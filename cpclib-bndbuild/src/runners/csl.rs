use std::marker::PhantomData;

use clap::{Arg, ArgAction, CommandFactory, Parser};
use cpclib_cslcli::CslCliArgs;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapDerive;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::built_info;
use crate::task::CSL_CMDS;

pub struct CslRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for CslRunner<E> {
    fn default() -> Self {
        let command = CslCliArgs::command()
            .no_binary_name(true)
            .bin_name(CSL_CMDS[0])
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
            .after_help(format!(
                "cpclib-cslcli {} embedded by {} {}",
                env!("CARGO_PKG_VERSION"),
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for CslRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapDerive for CslRunner<E> {
    type Args = CslCliArgs;
}

impl<E: EventObserver> Runner for CslRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let cli = self.get_args(itr, o)?;
        if cli.is_none() {
            return Ok(());
        }
        let cli = cli.unwrap();

        cpclib_cslcli::run(&cli).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        CSL_CMDS[0]
    }
}
