use std::marker::PhantomData;

use cpclib_common::clap::{self, Command};
use cpclib_runner::event::EventObserver;

use super::{Runner, RunnerWithClap};
use crate::built_info;
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
}

impl<E: EventObserver> Runner for BndBuildRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        // backup of cwd
        let cwd = std::env::current_dir().unwrap();

        // this will change the cwd
        let matches = self.get_matches(itr)?;
        crate::process_matches(&matches).map_err(|e| e.to_string())?;

        // restoration of cwd
        std::env::set_current_dir(cwd).unwrap();

        Ok(())
    }

    fn get_command(&self) -> &str {
        BNDBUILD_CMDS[0]
    }
}
