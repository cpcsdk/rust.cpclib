use std::io::Write;

use crate::error::AssemblerError;

use super::{delayed_command::*, report::SavedFile, save_command::SaveCommand, Env};
pub type ProtectedArea = std::ops::RangeInclusive<u16>;

/// Store all the compilation information for the currently selected 64kb page
/// A stock CPC 6128 is composed of two pages
#[derive(Debug, Clone)]
pub struct PageInformation {
    /// Start adr to use to write binary files. No use when working with snapshots.
    pub(crate) startadr: Option<u16>,
    /// maximum address reached when working with 64k data
    pub(crate) maxadr: u16,
    /// Current address to write to
    pub(crate) logical_outputadr: u16,
    /// Current address used by the code
    pub(crate) logical_codeadr: u16,
    /// Maximum possible address to write to
    pub(crate) limit: u16,
    /// List of pretected zones
    pub(crate) protected_areas: Vec<ProtectedArea>,
    pub(crate) fail_next_write_if_zero: bool,

    /// List of save commands  that will be executed ONLY after full assembling (they are emptied at the beginning of each pass)
    delayed_commands: DelayedCommands,
}

impl Default for PageInformation {
    fn default() -> Self {
        Self {
            startadr: None,
            maxadr: 0,
            logical_outputadr: 0,
            logical_codeadr: 0,
            limit: 0xffff,
            protected_areas: Vec::new(),
            fail_next_write_if_zero: false,
            delayed_commands: DelayedCommands::default(),
        }
    }
}

impl PageInformation {
    /// Properly set the information for a new pass
    pub fn new_pass(&mut self) {
        self.startadr = None;
        self.maxadr = 0;
        self.logical_outputadr = 0;
        self.logical_codeadr = 0;
        self.limit = 0xffff;
        self.fail_next_write_if_zero = false;
        self.delayed_commands.clear();
    }

    delegate::delegate! {
        to self.delayed_commands {
            pub fn add_save_command(&mut self, command: SaveCommand);
            pub fn add_failed_assert_command(&mut self, command: FailedAssertCommand);
            pub fn add_print_command(&mut self, command: PrintCommand);
            pub fn add_pause_command(&mut self, command: PauseCommand);
            pub fn add_print_or_pause_command(&mut self, command: PrintOrPauseCommand);

            pub fn print_commands(&self) -> &[PrintOrPauseCommand];
            pub fn print_commands_mut(&mut self) -> &mut [PrintOrPauseCommand];

            pub fn failed_assert_commands(&self) -> &[FailedAssertCommand] ;
            pub fn failed_assert_commands_mut(&mut self) -> &mut[FailedAssertCommand] ;


            pub fn execute_save(&self, env: &Env) -> Result<Vec<SavedFile>, AssemblerError>;
            pub fn collect_assert_failure(&self) -> Result<(), AssemblerError>;
            pub fn execute_print_or_pause(&self, writer: &mut impl Write)-> Result<(), AssemblerError>;
        }

    }
}
