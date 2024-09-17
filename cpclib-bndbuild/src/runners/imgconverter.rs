use std::fmt::Display;
use std::marker::PhantomData;
use std::rc::Rc;

use cpclib_common::clap::{Arg, ArgAction, Command};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::task::IMG2CPC_CMDS;

pub struct ImgConverterRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for ImgConverterRunner<E> {
    fn default() -> Self {
        let command = cpclib_imgconverter::build_args_parser()
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_imgconverter::built_info::PKG_NAME,
                cpclib_imgconverter::built_info::PKG_VERSION,
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
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
                    .exclusive(true)
            )
            .no_binary_name(true);
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for ImgConverterRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> Runner for ImgConverterRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let args = self.get_clap_command().clone();

        // let itr = Some(self.get_command()).into_iter().chain(itr.iter().map(|s| s.as_ref())).collect_vec(); XXX done

        let matches = self.get_matches(&itr)?;
        if matches.get_flag("version") {
            o.emit_stdout(&self.get_clap_command().clone().render_version());
            return Ok(());
        }

        if matches.get_flag("help") {
            o.emit_stdout(
                &self
                    .get_clap_command()
                    .clone()
                    .render_long_help()
                    .to_string()
            );
            return Ok(());
        }

        cpclib_imgconverter::process(&matches, args).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        IMG2CPC_CMDS[0]
    }
}
