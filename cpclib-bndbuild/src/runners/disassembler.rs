use std::fmt::Debug;

use cpclib_bdasm::{BdAsmCli, process};
use cpclib_common::event::EventObserver;
use cpclib_runner::runner::disassembler::ExternDisassembler;
use cpclib_runner::runner::runner::RunnerWithClapDerive;
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
crate::define_clap_derive_runner! {
    BdasmRunner,
    BdAsmCli,
    BDASM_CMDS[0],
    cpclib_bdasm::built_info::PKG_VERSION,
    |cli: BdAsmCli| { process(&cli).map_err(|e| e.to_string()) }
}
