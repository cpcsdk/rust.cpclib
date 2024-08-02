use cpclib_common::clap::{Arg, ArgAction, Command};

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::IMG2CPC_CMDS;

pub struct ImgConverterRunner {
    command: Command
}

impl Default for ImgConverterRunner {
    fn default() -> Self {
        let command = cpclib_imgconverter::build_args_parser()
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_imgconverter::built_info::PKG_NAME,
                cpclib_imgconverter::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            //  .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
            )
            .no_binary_name(true);
        Self { command }
    }
}

impl RunnerWithClap for ImgConverterRunner {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl Runner for ImgConverterRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let args = self.get_clap_command().clone();

        let matches = self.get_matches(itr)?;
        if matches.get_flag("version") {
            println!("{}", self.get_clap_command().clone().render_version());
            return Ok(());
        }

        cpclib_imgconverter::process(&matches, args).map_err(|e| dbg!(e).to_string())
    }

    fn get_command(&self) -> &str {
        &IMG2CPC_CMDS[0]
    }
}
