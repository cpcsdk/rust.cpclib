use std::marker::PhantomData;

use clap::{Arg, ArgAction, Command};
use cpclib_common::event::EventObserver;
use cpclib_crunch::CrunchArgs;
use cpclib_runner::runner::runner::RunnerWithClapDerive;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::CRUNCH_CMDS;

pub struct CrunchRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for CrunchRunner<E> {
    fn default() -> Self {
        let command = cpclib_crunch::command();
        let command = command
            .no_binary_name(true)
            .bin_name(CRUNCH_CMDS[0])
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
                    .exclusive(true) // does not seem to work
            )
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_crunch::built_info::PKG_NAME,
                cpclib_crunch::built_info::PKG_VERSION,
                crate::built_info::PKG_NAME,
                crate::built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for CrunchRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapDerive for CrunchRunner<E> {
    type Args = CrunchArgs;
}

impl<E: EventObserver> Runner for CrunchRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_args(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        let start = std::time::Instant::now();

        cpclib_crunch::process(matches)
    }

    fn get_command(&self) -> &str {
        CRUNCH_CMDS[0]
    }
}
