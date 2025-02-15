use std::fmt::Debug;
use std::marker::PhantomData;

use clap::{Arg, ArgAction, Command};
use cpclib_bdasm::process;
use cpclib_common::event::EventObserver;
use cpclib_common::itertools::Itertools;
use cpclib_runner::runner::disassembler::ExternDisassembler;
use cpclib_runner::runner::runner::RunnerWithClapMatches;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::BDASM_CMDS;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Disassembler {
    Bdasm,
    Extern(ExternDisassembler)
}

impl Disassembler {
    pub fn get_command(&self) -> &str {
        match self {
            Disassembler::Bdasm => BDASM_CMDS[0],
            Disassembler::Extern(d) => d.get_command()
        }
    }
}

pub struct BdasmRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for BdasmRunner<E> {
    fn default() -> Self {
        let command = cpclib_bdasm::build_args_parser();
        let command = command
            .no_binary_name(true)
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
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_bdasm::built_info::PKG_NAME,
                cpclib_bdasm::built_info::PKG_VERSION,
                crate::built_info::PKG_NAME,
                crate::built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for BdasmRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for BdasmRunner<E> {
}

impl<E: EventObserver> Runner for BdasmRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        let matches = self.get_matches(&itr, o)?;

        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        process(&matches);
        Ok(())
    }

    fn get_command(&self) -> &str {
        BDASM_CMDS[0]
    }
}
