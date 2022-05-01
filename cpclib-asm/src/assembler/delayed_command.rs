use std::io::Write;

use codespan_reporting::diagnostic::Severity;
use cpclib_common::itertools::Itertools;
#[cfg(not(target_arch = "wasm32"))]
use cpclib_common::rayon::prelude::*;

use super::report::SavedFile;
use super::save_command::SaveCommand;
use super::Env;
use crate::error::{build_simple_error_message, AssemblerError};
use crate::preamble::Z80Span;
trait DelayedCommand {}

#[derive(Debug, Clone)]
pub struct PrintCommand {
    pub(crate) span: Option<Z80Span>,
    pub(crate) print_or_error: either::Either<String, AssemblerError>
}

impl PrintCommand {
    pub fn relocate(&mut self, span: Z80Span) {
        self.span.replace(span);
    }
}
#[derive(Debug, Clone)]
pub struct FailedAssertCommand {
    failure: AssemblerError
}

/// Expect an assert error or a exval error
impl From<AssemblerError> for FailedAssertCommand {
    fn from(failure: AssemblerError) -> Self {
        Self { failure }
    }
}

impl DelayedCommand for PrintCommand {}

impl DelayedCommand for FailedAssertCommand {}

impl PrintCommand {
    pub fn execute(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        match &self.print_or_error {
            either::Either::Left(msg) => {
                // TODO improve printting + integrate z80span information
                write!(
                    writer,
                    "{}",
                    if let Some(span) = &self.span {
                        build_simple_error_message(msg, &span, Severity::Note)
                    }
                    else {
                        msg.to_owned()
                    }
                )
                .unwrap();
                Ok(())
            }
            either::Either::Right(e) => Err(e.clone())
        }
    }
}
#[derive(Debug, Clone)]

pub struct PauseCommand(Option<Z80Span>);

impl From<Option<Z80Span>> for PauseCommand {
    fn from(s: Option<Z80Span>) -> Self {
        Self(s)
    }
}

impl PauseCommand {
    pub fn execute(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        let msg = "PAUSE - press enter to continue.";
        write!(
            writer,
            "{}",
            if let Some(span) = &self.0 {
                build_simple_error_message(msg, &span, Severity::Note)
            }
            else {
                msg.to_owned()
            }
        )
        .unwrap();

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();
        Ok(())
    }

    pub fn relocate(&mut self, span: Z80Span) {
        self.0.replace(span);
    }
}

#[derive(Debug, Clone)]
pub enum PrintOrPauseCommand {
    Print(PrintCommand),
    Pause(PauseCommand)
}

impl From<PrintCommand> for PrintOrPauseCommand {
    fn from(p: PrintCommand) -> Self {
        PrintOrPauseCommand::Print(p)
    }
}

impl From<PauseCommand> for PrintOrPauseCommand {
    fn from(p: PauseCommand) -> Self {
        PrintOrPauseCommand::Pause(p)
    }
}

impl PrintOrPauseCommand {
    pub fn execute(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        match self {
            PrintOrPauseCommand::Print(p) => p.execute(writer),
            PrintOrPauseCommand::Pause(p) => p.execute(writer)
        }
    }

    pub fn relocate(&mut self, span: Z80Span) {
        match self {
            PrintOrPauseCommand::Print(p) => p.relocate(span),
            PrintOrPauseCommand::Pause(p) => p.relocate(span)
        }
    }
}

/// Information for a breakpoint:
/// TODO: add condition
#[derive(Debug, Clone)]
pub struct BreakpointCommand {
    pub(crate) address: u16,
    pub(crate) page: u8,
    pub(crate) span: Z80Span
}

impl BreakpointCommand {
    pub fn new(address: u16, page: u8, span: Z80Span) -> Self {
        BreakpointCommand {
            address,
            page,
            span
        }
    }

    pub fn winape_raw(&self) -> [u8; 5] {
        [
            (self.address & 0xFF) as u8,
            (self.address >> 8) as u8,
            self.page,
            0,
            0
        ]
    }
}

#[derive(Debug, Clone)]
pub struct DelayedCommands {
    failed_assert_commands: Vec<FailedAssertCommand>,
    save_commands: Vec<SaveCommand>,
    print_commands: Vec<PrintOrPauseCommand>,
    breakpoint_commands: Vec<BreakpointCommand>
}

impl Default for DelayedCommands {
    fn default() -> Self {
        Self {
            failed_assert_commands: Vec::new(),
            save_commands: Vec::new(),
            print_commands: Vec::new(),
            breakpoint_commands: Vec::new()
        }
    }
}

impl DelayedCommands {
    pub fn clear(&mut self) {
        self.failed_assert_commands.clear();
        self.save_commands.clear();
        self.print_commands.clear();
        self.breakpoint_commands.clear();
    }
}

/// Commands addition
impl DelayedCommands {
    pub fn add_breakpoint_command(&mut self, command: BreakpointCommand) {
        self.breakpoint_commands.push(command);
    }

    pub fn add_save_command(&mut self, command: SaveCommand) {
        self.save_commands.push(command);
    }

    pub fn add_failed_assert_command(&mut self, command: FailedAssertCommand) {
        self.failed_assert_commands.push(command);
    }

    pub fn add_print_command(&mut self, command: PrintCommand) {
        self.add_print_or_pause_command(command.into());
    }

    pub fn add_pause_command(&mut self, command: PauseCommand) {
        self.add_print_or_pause_command(command.into());
    }

    pub fn add_print_or_pause_command(&mut self, command: PrintOrPauseCommand) {
        self.print_commands.push(command)
    }
}

/// Commands execution
impl DelayedCommands {
    pub fn execute_save(&self, env: &Env) -> Result<Vec<SavedFile>, AssemblerError> {
        #[cfg(not(target_arch = "wasm32"))]
        let iter = self.save_commands.par_iter();
        #[cfg(target_arch = "wasm32")]
        let iter = self.save_commands.iter();

        let res = iter
            .map(|cmd| cmd.execute_on(env))
            .collect::<Result<Vec<_>, AssemblerError>>()?;

        Ok(res)
    }

    /// Return Ok if no assertion error, Err otherwise
    pub fn collect_assert_failure(&self) -> Result<(), AssemblerError> {
        if self.failed_assert_commands.is_empty() {
            Ok(())
        }
        else {
            Err(AssemblerError::MultipleErrors {
                errors: self
                    .failed_assert_commands
                    .iter()
                    .map(|a| a.failure.clone())
                    .collect_vec()
            })
        }
    }

    pub fn execute_print_or_pause(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        // todo aggregate successive print to write them in one time
        let res = self
            .print_commands
            .iter()
            .map(|p| p.execute(writer))
            .filter(|r| r.is_err())
            .map(|e| e.err().unwrap())
            .collect_vec();
        if res.is_empty() {
            Ok(())
        }
        else {
            Err(AssemblerError::MultipleErrors { errors: res })
        }
    }
}

impl DelayedCommands {
    pub fn print_commands(&self) -> &[PrintOrPauseCommand] {
        &self.print_commands
    }

    pub fn print_commands_mut(&mut self) -> &mut [PrintOrPauseCommand] {
        &mut self.print_commands
    }

    pub fn failed_assert_commands(&self) -> &[FailedAssertCommand] {
        &self.failed_assert_commands
    }

    pub fn failed_assert_commands_mut(&mut self) -> &mut [FailedAssertCommand] {
        &mut self.failed_assert_commands
    }
}

impl DelayedCommands {
    pub fn collect_breakpoints(&self) -> &[BreakpointCommand] {
        &self.breakpoint_commands
    }
}
