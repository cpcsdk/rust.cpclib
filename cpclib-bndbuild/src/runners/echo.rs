use cpclib_common::itertools::Itertools;

use crate::task::ECHO_CMDS;

use super::Runner;

#[derive(Default)]
pub struct EchoRunner {}

impl Runner for EchoRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let txt = itr.iter().map(|s| s.as_ref()).join(" ");
        println!("{txt}");
        Ok(())
    }

    fn get_command(&self) -> &str {
        &ECHO_CMDS[0]
    }
}
