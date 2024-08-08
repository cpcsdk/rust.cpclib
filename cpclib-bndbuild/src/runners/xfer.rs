use cpclib_common::clap::{self, Arg, ArgAction, Command};

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::XFER_CMDS;

pub struct XferRunner {
    command: clap::Command
}

impl Default for XferRunner {
    fn default() -> Self {
        let command = cpclib_xfertool::build_args_parser();
        let command = command
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
            )
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_xfertool::built_info::PKG_NAME,
                cpclib_xfertool::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self { command }
    }
}

impl RunnerWithClap for XferRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for XferRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let matches = self.get_matches(itr)?;

        if matches.get_flag("version") {
            println!("{}", self.get_clap_command().clone().render_version());
            return Ok(());
        }

        cpclib_xfertool::process(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        XFER_CMDS[0]
    }
}
