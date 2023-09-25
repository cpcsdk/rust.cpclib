use cpclib_common::clap::{Command, self};
use cpclib_disc::dsk_manager_build_arg_parser;

use super::{RunnerWithClap, Runner};
use crate::built_info;


pub struct DskManagerRunner {
    command: clap::Command
}

impl Default for DskManagerRunner {
    fn default() -> Self {
        let command = dsk_manager_build_arg_parser();
        let command = command
            .no_binary_name(true)
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_disc::built_info::PKG_NAME,
                cpclib_disc::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self{command}
    }
}

impl RunnerWithClap for DskManagerRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for DskManagerRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let matches = self.get_matches(itr)?;
        cpclib_disc::dsk_manager_handle(matches)
            .map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        "dsk"
    }
}
