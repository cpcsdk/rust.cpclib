use cpclib_common::clap::{ArgMatches, Command};
use cpclib_runner::runner::Runner;

pub mod assembler;
pub mod bndbuild;
pub mod cp;
pub mod disc;
pub mod echo;
pub mod imgconverter;
pub mod rm;
pub mod xfer;

pub trait RunnerWithClap: Runner {
    fn get_clap_command(&self) -> &Command;

    fn get_matches<S: AsRef<str>>(&self, itr: &[S]) -> Result<ArgMatches, String> {
        self.get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
            .map_err(|e| e.to_string())
    }

    fn print_help(&self) {
        self.get_clap_command()
            .clone()
            .disable_help_flag(true)
            .print_long_help()
            .unwrap();
    }
}
