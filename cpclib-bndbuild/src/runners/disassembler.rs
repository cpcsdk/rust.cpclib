use std::fmt::Debug;

use cpclib_bdasm::process;
use cpclib_common::event::EventObserver;
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

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner_simple! {
    BdasmRunner,
    cpclib_bdasm::build_args_parser(),
    BDASM_CMDS[0],
    cpclib_bdasm::built_info::PKG_NAME,
    cpclib_bdasm::built_info::PKG_VERSION,
    |matches| { process(&matches); Ok(()) }
}
