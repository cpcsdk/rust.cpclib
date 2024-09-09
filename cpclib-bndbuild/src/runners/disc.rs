use std::marker::PhantomData;

use cpclib_common::clap::{self, Command};
use cpclib_disc::dsk_manager_build_arg_parser;
use cpclib_runner::event::EventObserver;

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
        let command = command.no_binary_name(true).after_help(format!(
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

impl<E: EventObserver> Runner for DiscManagerRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr)?;
        cpclib_disc::dsk_manager_handle(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        DISC_CMDS[0]
    }
}
