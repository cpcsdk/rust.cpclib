use std::collections::BTreeMap;
use std::io::Write;

use codespan_reporting::diagnostic::Severity;
use cpclib_common::itertools::Itertools;
use cpclib_sna::{AceBreakPoint, AceBrkRuntimeMode, RemuEntry, WinapeBreakPoint};
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use {cpclib_common::rayon::prelude::*, rayon_cond::CondIterator};

use super::report::SavedFile;
use super::save_command::SaveCommand;
use super::string::PreprocessedFormattedString;
use super::Env;
use crate::error::{build_simple_error_message, AssemblerError};
use crate::preamble::Z80Span;

trait DelayedCommand {}

#[derive(Debug, Clone)]
pub struct PrintCommand {
    pub(crate) prefix: Option<String>,
    pub(crate) span: Option<Z80Span>,
    pub(crate) print_or_error: either::Either<PreprocessedFormattedString, AssemblerError>
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
    #[inline]
    pub fn string_or_error(&self) -> Result<String, AssemblerError> {
        match &self.print_or_error {
            either::Either::Left(msg) => {
                // TODO improve printting + integrate z80span information
                let file_location = if let Some(span) = &self.span {
                    let fname = span.filename();
                    let (line, col) = span.relative_line_and_column();

                    Some((fname, line, col))
                }
                else {
                    None
                };

                // duplicate code to speed it up
                let repr = match (&self.prefix, file_location) {
                    (Some(prefix), Some(loc)) => {
                        format!("{}{}:{}:{} PRINT: {}", prefix, loc.0, loc.1, loc.2, msg)
                    },

                    (Some(prefix), None) => {
                        format!("{} PRINT: {}", prefix, msg)
                    },

                    (None, Some(loc)) => {
                        format!("{}:{}:{} PRINT: {}", loc.0, loc.1, loc.2, msg)
                    },

                    (None, None) => {
                        format!("PRINT: {}", msg)
                    }
                };

                Ok(repr)
            },
            either::Either::Right(e) => Err(e.clone())
        }
    }

    // XXX The code is the same than string_or_error
    #[inline]
    pub fn execute(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        match &self.print_or_error {
            either::Either::Left(msg) => {
                // TODO improve printting + integrate z80span information
                let file_location = if let Some(span) = &self.span {
                    let fname = span.filename();
                    let (line, col) = span.relative_line_and_column();

                    Some((fname, line, col))
                }
                else {
                    None
                };

                // duplicate code to speed it up
                match (&self.prefix, file_location) {
                    (Some(prefix), Some(loc)) => {
                        writeln!(
                            writer,
                            "{}{}:{}:{} PRINT: {}",
                            prefix, loc.0, loc.1, loc.2, msg
                        )
                    },

                    (Some(prefix), None) => {
                        writeln!(writer, "{} PRINT: {}", prefix, msg)
                    },

                    (None, Some(loc)) => {
                        writeln!(writer, "{}:{}:{} PRINT: {}", loc.0, loc.1, loc.2, msg)
                    },

                    (None, None) => {
                        writeln!(writer, "PRINT: {}", msg)
                    }
                };

                Ok(())
            },
            either::Either::Right(e) => Err(e.clone())
        }
    }

    #[inline]
    pub fn is_print(&self) -> bool {
        self.print_or_error.is_left()
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
    #[inline]
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
    pub(crate) span: Option<Z80Span>
}

impl BreakpointCommand {
    pub fn new(address: u16, page: u8, span: Option<Z80Span>) -> Self {
        BreakpointCommand {
            address,
            page,
            span
        }
    }

    pub fn winape(&self) -> WinapeBreakPoint {
        WinapeBreakPoint::new(self.address, self.page)
    }

    pub fn ace(&self) -> AceBreakPoint {
        AceBreakPoint::new_execution(
            self.address,
            AceBrkRuntimeMode::Break,
            cpclib_sna::AceMemMapType::Undefined
        )
    }

    pub fn remu(&self) -> RemuEntry {
        RemuEntry::BreakPoint(self.address, self.page)
    }
}

#[derive(Debug, Clone)]
pub struct DelayedCommands {
    failed_assert_commands: Vec<FailedAssertCommand>,
    save_commands: BTreeMap<u8, Vec<SaveCommand>>, // commands are ordered per ga_mmr
    print_commands: Vec<PrintOrPauseCommand>,
    breakpoint_commands: Vec<BreakpointCommand>
}

impl Default for DelayedCommands {
    fn default() -> Self {
        Self {
            failed_assert_commands: Vec::new(),
            save_commands: Default::default(),
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
        self.save_commands
            .entry(command.ga_mmr())
            .or_default()
            .push(command);
    }

    pub fn get_save_mmrs(&self) -> Vec<u8> {
        self.save_commands.keys().cloned().collect_vec()
    }

    /// can save in parallel if all commands can be saved in parallel (we are strict because we miss lots of parallelism)
    pub fn can_save_in_parallel(&self) -> bool {
        self.save_commands
            .values()
            .all(|s| s.iter().all(|s| s.can_be_saved_in_parallel()))
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
    /// Execute the commands that correspond to the appropriate mmr configuration
    pub fn execute_save(&self, env: &Env, ga_mmr: u8) -> Result<Vec<SavedFile>, AssemblerError> {
        #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
        let iter = CondIterator::new(&self.save_commands, self.can_save_in_parallel());
        #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
        let iter = self.save_commands.iter();

        let res = iter
            .filter_map(|(save_mmr, save_cmd)| {
                if *save_mmr == ga_mmr {
                    Some(save_cmd)
                }
                else {
                    None
                }
            })
            .flatten()
            .map(|cmd| cmd.execute_on(env))
            .collect::<Result<Vec<_>, AssemblerError>>()?;

        Ok(res)
    }

    pub fn nb_files_to_save(&self) -> usize {
        self.save_commands.len()
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

    /// XXX Current version completly ignore pause. TODO find a way to reactivate
    pub fn execute_print_or_pause(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        // Generate in parallal all the strings (I doubt it is efficient as there is a lock...)

        #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
        let iter = self.print_commands.par_iter();

        #[cfg(not(all(not(target_arch = "wasm32"), feature = "rayon")))]
        let iter = self.print_commands.iter();

        let errors = iter
            .filter_map(|c| {
                match c {
                    PrintOrPauseCommand::Print(p) => {
                        if p.is_print() {
                            p.execute(writer);
                            None
                        }
                        else {
                            Some(p.print_or_error.as_ref().right().unwrap().clone())
                        }
                    },
                    PrintOrPauseCommand::Pause(p) => {
                        p.execute(writer);
                        None
                    }
                }
            })
            .collect_vec();

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(AssemblerError::MultipleErrors { errors })
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
