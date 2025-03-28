use std::marker::PhantomData;

use cpclib_common::clap::{self, Arg, ArgAction, Command};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::XFER_CMDS;

pub struct XferRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for XferRunner<E> {
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
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .exclusive(true) // does not seem to work
            )
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_xfertool::built_info::PKG_NAME,
                cpclib_xfertool::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for XferRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for XferRunner<E> {}

impl<E: EventObserver> Runner for XferRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr, o)?;

        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        cpclib_xfertool::process(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        XFER_CMDS[0]
    }
}
