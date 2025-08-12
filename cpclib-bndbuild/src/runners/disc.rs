use std::marker::PhantomData;

use clap::{Arg, ArgAction};
use cpclib_common::clap::{self, Command};
use cpclib_disc::dsk_manager_build_arg_parser;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::DISC_CMDS;

pub struct DiscManagerRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for DiscManagerRunner<E> {
    fn default() -> Self {
        let command = dsk_manager_build_arg_parser();
        let command = command
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
            .no_binary_name(true)
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_disc::built_info::PKG_NAME,
                cpclib_disc::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for DiscManagerRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for DiscManagerRunner<E> {}

impl<E: EventObserver> Runner for DiscManagerRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        cpclib_disc::dsk_manager_handle(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        DISC_CMDS[0]
    }
}
