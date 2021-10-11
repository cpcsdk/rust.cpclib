use std::io::Write;

use super::{report::SavedFile, save_command::SaveCommand, Env};
use crate::error::build_simple_error_message;
use crate::{error::AssemblerError, preamble::Z80Span};
use codespan_reporting::diagnostic::Severity;
use cpclib_common::itertools::Itertools;
use cpclib_common::rayon::prelude::*;
trait DelayedCommand {}

#[derive(Debug, Clone)]
pub struct PrintCommand {
    pub(crate) span: Option<Z80Span>,
    pub(crate) print_or_error: either::Either<String, AssemblerError>,
}

impl PrintCommand {
    pub fn relocate(&mut self, span: Z80Span) {
        self.span.replace(span);
    }
}
#[derive(Debug, Clone)]
pub struct FailedAssertCommand {
    failure: AssemblerError,
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
                    } else {
                        msg.to_owned()
                    }
                ).unwrap();
                Ok(())
            }
            either::Either::Right(e) => Err(e.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DelayedCommands {
    failed_assert_commands: Vec<FailedAssertCommand>,
    save_commands: Vec<SaveCommand>,
    print_commands: Vec<PrintCommand>,
}

impl Default for DelayedCommands {
    fn default() -> Self {
        Self {
            failed_assert_commands: Vec::new(),
            save_commands: Vec::new(),
            print_commands: Vec::new(),
        }
    }
}

impl DelayedCommands {
    pub fn clear(&mut self) {
        self.failed_assert_commands.clear();
        self.save_commands.clear();
        self.print_commands.clear();
    }
}

/// Commands addition
impl DelayedCommands {
    pub fn add_save_command(&mut self, command: SaveCommand) {
        self.save_commands.push(command);
    }

    pub fn add_failed_assert_command(&mut self, command: FailedAssertCommand) {
        self.failed_assert_commands.push(command);
    }

    pub fn add_print_command(&mut self, command: PrintCommand) {
        self.print_commands.push(command);
    }
}

/// Commands execution
impl DelayedCommands {
    pub fn execute_save(&self, env: &Env) -> Result<Vec<SavedFile>, AssemblerError> {
        let res = self
            .save_commands
            .par_iter()
            .map(|cmd| cmd.execute_on(env))
            .collect::<Result<Vec<_>, AssemblerError>>()?;

        Ok(res)
    }

    /// Return Ok if no assertion error, Err otherwise
    pub fn collect_assert_failure(&self) -> Result<(), AssemblerError> {
        if self.failed_assert_commands.is_empty() {
            Ok(())
        } else {
            Err(AssemblerError::MultipleErrors {
                errors: self
                    .failed_assert_commands
                    .iter()
                    .map(|a| a.failure.clone())
                    .collect_vec(),
            })
        }
    }

    pub fn execute_print(&self, writer: &mut impl Write) -> Result<(), AssemblerError> {
        let res = self
            .print_commands
            .iter()
            .map(|p| p.execute(writer))
            .filter(|r| r.is_err())
            .map(|e| e.err().unwrap())
            .collect_vec();
        if res.is_empty() {
            Ok(())
        } else {
            Err(AssemblerError::MultipleErrors { errors: res })
        }
    }
}


impl DelayedCommands {
    pub fn print_commands(&self) -> &[PrintCommand] {
        & self.print_commands
    }
    pub fn print_commands_mut(&mut self) -> &mut [PrintCommand] {
        &mut self.print_commands
    }

}