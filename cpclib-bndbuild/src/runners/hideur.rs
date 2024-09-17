use std::marker::PhantomData;
use std::rc::Rc;

use cpclib_disc::hideur::{hideur_build_arg_parser, hideur_handle};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::built_info;
use crate::task::HIDEUR_CMDS;

pub const HIDEUR_CMD: &str = "hideur";

pub struct HideurRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for HideurRunner<E> {
    fn default() -> Self {
        let command = hideur_build_arg_parser();
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

impl<E: EventObserver> RunnerWithClap for HideurRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> Runner for HideurRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr)?;
        hideur_handle(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        HIDEUR_CMDS[0]
    }
}
