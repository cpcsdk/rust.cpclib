use std::marker::PhantomData;

use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::{Runner, RunnerWithClap};
use cpclib_sna;

use crate::built_info;
use crate::task::SNA_CMDS;

pub const SNAPSHOT_CMD: &str = "SNAPSHOT";

pub struct SnapshotRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for SnapshotRunner<E> {
    fn default() -> Self {
        let command = cpclib_sna::build_arg_parser();
        let command = command.no_binary_name(true).after_help(format!(
            "{} {} embedded by {} {}",
            cpclib_sna::built_info::PKG_NAME,
            cpclib_sna::built_info::PKG_VERSION,
            built_info::PKG_NAME,
            built_info::PKG_VERSION
        ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for SnapshotRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> Runner for SnapshotRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr)?;
        cpclib_sna::process(&matches, o).map_err(|e| format!("{:?}", e))
    }

    fn get_command(&self) -> &str {
        SNA_CMDS[0]
    }
}
