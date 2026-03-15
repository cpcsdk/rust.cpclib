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
    o:
    BdasmRunner,
    BdAsmCli,
    BDASM_CMDS[0],
    cpclib_bdasm::built_info::PKG_VERSION,
    |cli: BdAsmCli, o| { process(&cli, o).map_err(|e| e.to_string()) }
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_bdasm_help_flag_captured() {
        let runner = super::BdasmRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_bdasm_version_flag_captured() {
        let runner = super::BdasmRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_bdasm_invalid_arg_captured() {
        let runner = super::BdasmRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
