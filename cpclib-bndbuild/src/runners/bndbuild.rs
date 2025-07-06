use std::marker::PhantomData;

use clap::ArgMatches;
use cpclib_common::clap::{self, Command};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::event::BndBuilderObserverRc;
use crate::task::BNDBUILD_CMDS;

pub struct BndBuildRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for BndBuildRunner<E> {
    fn default() -> Self {
        let command = crate::build_args_parser();
        let command = command.no_binary_name(true).after_help(format!(
            "{} {} embedded by {} {}",
            crate::built_info::PKG_NAME,
            crate::built_info::PKG_VERSION,
            built_info::PKG_NAME,
            built_info::PKG_VERSION
        ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for BndBuildRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }

    fn get_matches<S: AsRef<str>>(
        &self,
        itr: &[S],
        e: &dyn EventObserver
    ) -> Result<Option<ArgMatches>, String> {
        let args = self
            .get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
            .map_err(|e| e.to_string())?;

        { Ok(Some(args)) }
    }
}

impl<E: EventObserver> RunnerWithClapMatches for BndBuildRunner<E> {}

impl<E: EventObserver> Runner for BndBuildRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String> {
        // backup of cwd
        let cwd = std::env::current_dir().unwrap();

        // this will change the cwd
        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        // BUG here we skip the observer. It is necessary to find a way to use it properly
        let o = BndBuilderObserverRc::new_default();
        let res = crate::process_matches_with_observer(&matches, o);
        // restoration of cwd
        std::env::set_current_dir(cwd).unwrap();

        res.map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        BNDBUILD_CMDS[0]
    }
}
