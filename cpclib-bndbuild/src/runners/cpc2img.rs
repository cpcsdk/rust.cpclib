use std::marker::PhantomData;

use cpclib_common::clap::{Arg, ArgAction, Command};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::CPC2IMG_CMDS;

pub struct CpcToImgRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for CpcToImgRunner<E> {
    fn default() -> Self {
        let command = cpclib_imgconverter::build_cpc2img_args_parser()
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_imgconverter::built_info::PKG_NAME,
                cpclib_imgconverter::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .exclusive(true) // does not seem to work
            )
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
            )
            .no_binary_name(true);
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for CpcToImgRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for CpcToImgRunner<E> {}

impl<E: EventObserver> Runner for CpcToImgRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let args = self.get_clap_command().clone();

        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        cpclib_imgconverter::process_img2cpc(&matches, args).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        CPC2IMG_CMDS[0]
    }
}
