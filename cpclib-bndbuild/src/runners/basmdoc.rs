pub const BASMDOC_CMD: &str = "basmdoc";

use std::marker::PhantomData;

use clap::{ArgMatches, Command};
use cpclib_common::event::EventObserver;
use cpclib_runner::runner::{Runner, RunnerWithClap, runner::RunnerWithClapMatches};

use crate::task::BASMDOC_CMDS;

pub struct BasmDocRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for BasmDocRunner<E> {
    fn default() -> Self {
        let command = cpclib_basmdoc::cmdline::build_args_parser();
        let command = command.no_binary_name(true);

        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for BasmDocRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }

    fn get_matches<S: AsRef<str>>(
        &self,
        itr: &[S],
        _e: &dyn EventObserver
    ) -> Result<Option<ArgMatches>, String> {
        let args = self
            .get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
            .map_err(|e| e.to_string())?;

        Ok(Some(args))
    }
}

impl<E: EventObserver> RunnerWithClapMatches for BasmDocRunner<E> {}

impl<E: EventObserver> Runner for BasmDocRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String> {

        let matches = self.get_matches(itr, o)?;
        let matches = matches.unwrap();

		let res = cpclib_basmdoc::cmdline::handle_matches(&matches);

        res.map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        BASMDOC_CMD
    }
}
