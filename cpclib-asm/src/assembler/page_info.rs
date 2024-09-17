

use super::delayed_command::*;
use super::report::SavedFile;
use super::save_command::SaveCommand;
use super::{Env, EnvEventObserver};
use crate::error::AssemblerError;
pub type ProtectedArea = std::ops::RangeInclusive<u16>;

/// Store all the compilation information for the currently selected 64kb page
/// A stock CPC 6128 is composed of two pages
#[derive(Debug, Clone)]
pub struct PageInformation {
    /// Start adr to use to write binary files. Not use when working with snapshots.
    pub(crate) startadr: Option<u16>,
    /// maximum address reached when working with 64k data
    pub(crate) maxadr: u16,
    /// Current address to write to
    pub(crate) logical_outputadr: u16,
    /// Current address used by the code
    pub(crate) logical_codeadr: u16,
    /// Maximum possible address to write to
    pub(crate) output_limit: u16,
    pub(crate) code_limit: u16,
    /// List of pretected zones
    pub(crate) protected_areas: Vec<ProtectedArea>,
    pub(crate) fail_next_write_if_zero: bool,

    /// List of save commands  that will be executed ONLY after full assembling (they are emptied at the beginning of each pass)
    delayed_commands: DelayedCommands
}

impl Default for PageInformation {
    fn default() -> Self {
        Self {
            startadr: None,
            maxadr: 0,
            logical_outputadr: 0,
            logical_codeadr: 0,
            output_limit: 0xFFFF,
            code_limit: 0xFFFF,
            protected_areas: Vec::new(),
            fail_next_write_if_zero: false,
            delayed_commands: DelayedCommands::default()
        }
    }
}

impl PageInformation {
    delegate::delegate! {
        to self.delayed_commands {
            pub fn add_breakpoint_command(&mut self, command: BreakpointCommand);

            pub fn add_save_command(&mut self, command: SaveCommand);
            pub fn add_failed_assert_command(&mut self, command: FailedAssertCommand);
            pub fn add_print_command(&mut self, command: PrintCommand);
            pub fn add_pause_command(&mut self, command: PauseCommand);
            pub fn add_print_or_pause_command(&mut self, command: PrintOrPauseCommand);

            pub fn print_commands(&self) -> &[PrintOrPauseCommand];
            pub fn print_commands_mut(&mut self) -> &mut [PrintOrPauseCommand];

            pub fn failed_assert_commands(&self) -> &[FailedAssertCommand] ;
            pub fn failed_assert_commands_mut(&mut self) -> &mut[FailedAssertCommand] ;

            pub fn can_save_in_parallel(&self) -> bool;
            pub fn get_save_mmrs(&self) -> Vec<u8>;
            pub fn execute_save(&self, env: &Env, mmr: u8) -> Result<Vec<SavedFile>, AssemblerError>;
            pub fn nb_files_to_save(&self) -> usize;
            pub fn collect_assert_failure(&self) -> Result<(), AssemblerError>;
            pub fn execute_print_or_pause(&self, o: &dyn EnvEventObserver)-> Result<(), AssemblerError>;
            pub fn collect_breakpoints(&self)-> &[BreakpointCommand];
        }

    }

    /// Properly set the information for a new pass
    pub fn new_pass(&mut self) {
        self.startadr = None;
        self.maxadr = 0;
        self.logical_outputadr = 0;
        self.logical_codeadr = 0;
        self.output_limit = 0xFFFF;
        self.fail_next_write_if_zero = false;
        self.delayed_commands.clear();
    }

    pub fn set_limit(&mut self, l: u16) -> Result<(), AssemblerError> {
        if l > self.output_limit {
            return Err(AssemblerError::AssemblingError {
                msg: format!(
                    "Cannot set a limit of &{:X} as a former limit of &{:X} is already set up",
                    l, self.output_limit
                )
            });
        }

        if l < self.maxadr {
            return Err(AssemblerError::AssemblingError {
                msg: format!(
                    "Cannot set a limit of &{:X} as some bytes has been written at &{:X}p",
                    l, self.maxadr
                )
            });
        }

        self.output_limit = l;
        Ok(())
    }
}
