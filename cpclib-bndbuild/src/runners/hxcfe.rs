use std::marker::PhantomData;

use clap::{Arg, ArgAction, CommandFactory, Parser};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapDerive;
use cpclib_runner::runner::{Runner, RunnerWithClap};
use hxcfe_cli::HxcfeCli;

use crate::built_info;
use crate::task::HXCFE_CMDS;

pub struct HxcfeRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for HxcfeRunner<E> {
    fn default() -> Self {
        let command = HxcfeCli::command()
            .no_binary_name(true)
            .bin_name(HXCFE_CMDS[0])
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
                "hxcfe_cli {} embedded by {} {}",
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

impl<E: EventObserver> RunnerWithClap for HxcfeRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapDerive for HxcfeRunner<E> {
    type Args = HxcfeCli;
}

impl<E: EventObserver> Runner for HxcfeRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let cli = self.get_args(itr, o)?;
        if cli.is_none() {
            return Ok(());
        }
        let cli = cli.unwrap();

        hxcfe_cli::run(&cli).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        HXCFE_CMDS[0]
    }
}
