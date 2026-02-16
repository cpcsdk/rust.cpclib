use std::marker::PhantomData;

use clap::{Arg, ArgAction, CommandFactory, Parser};
use cpclib_common::clap::Command;
use cpclib_locomotive::Cli;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::LOCOMOTIVE_CMDS;

pub struct LocomotiveRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for LocomotiveRunner<E> {
    fn default() -> Self {
        let command = Cli::command();
        let command = command
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
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
                "cpclib-locomotive embedded by {} {}",
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for LocomotiveRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for LocomotiveRunner<E> {}

impl<E: EventObserver> Runner for LocomotiveRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let cli = Cli::try_parse_from(
            [self.get_command().to_string()]
                .into_iter()
                .chain(itr.iter().map(|s| s.as_ref().to_string()))
        )
        .map_err(|e| e.to_string())?;

        cpclib_locomotive::handle_locomotive_arguments(cli).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        LOCOMOTIVE_CMDS[0]
    }
}
