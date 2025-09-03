use std::marker::PhantomData;

use clap::{Arg, ArgAction};
use cpclib_common::event::EventObserver;
use cpclib_imgconverter::{fade_build_args, fade_handle_matches};
use cpclib_runner::runner::{Runner, RunnerWithClap, runner::RunnerWithClapMatches};

use crate::task::FADE_CMDS;

pub const FADE_CMD: &str = "fade";

pub struct FadeRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for FadeRunner<E> {
    fn default() -> Self {
        let command = fade_build_args();
        let command = command
            .no_binary_name(true)
            .disable_help_flag(true)
            /*.after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_imgconverter::built_info::PKG_NAME,
                cpclib_imgconverter::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))*/
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
            );
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for FadeRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for FadeRunner<E> {}

impl<E: EventObserver> Runner for FadeRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        fade_handle_matches(&matches).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        FADE_CMDS[0]
    }
}
