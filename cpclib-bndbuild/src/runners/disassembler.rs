use std::fmt::Debug;

use cpclib_bdasm::process;
use cpclib_common::event::EventObserver;
use cpclib_runner::runner::disassembler::ExternDisassembler;
#[allow(unused_imports)]
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

    /// Returns all embedded disassembler variants (excludes Extern)
    pub fn all_embedded() -> impl Iterator<Item = Self> {
        [Self::Bdasm].into_iter()
    }

    /// Returns all disassembler variants including external ones
    pub fn all() -> impl Iterator<Item = Self> {
        Self::all_embedded().chain(ExternDisassembler::all().map(Self::Extern))
    }
}

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner! {
    simple: BdasmRunner,
    cpclib_bdasm::build_args_parser(),
    BDASM_CMDS[0],
    cpclib_bdasm::built_info::PKG_NAME,
    cpclib_bdasm::built_info::PKG_VERSION,
    |matches| { process(&matches); Ok(()) }
}
