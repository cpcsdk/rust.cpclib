use cpclib_common::clap::{self, Command};
use cpclib_disc::dsk_manager_build_arg_parser;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::DISC_CMDS;

pub struct DiscManagerRunner {
    command: clap::Command
}

impl Default for DiscManagerRunner {
    fn default() -> Self {
        let command = dsk_manager_build_arg_parser();
        let command = command.no_binary_name(true).after_help(format!(
            "{} {} embedded by {} {}",
            cpclib_disc::built_info::PKG_NAME,
            cpclib_disc::built_info::PKG_VERSION,
            built_info::PKG_NAME,
            built_info::PKG_VERSION
        ));
        Self { command }
    }
}

impl RunnerWithClap for DiscManagerRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for DiscManagerRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let matches = self.get_matches(itr)?;
        cpclib_disc::dsk_manager_handle(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        DISC_CMDS[0]
    }
}
