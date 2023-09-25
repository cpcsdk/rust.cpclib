use cpclib_common::itertools::Itertools;

use super::Runner;

#[derive(Default)]
pub struct EchoRunner {}

impl Runner for EchoRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let txt = itr.iter().join(" ");
        println!("{txt}");
        Ok(())
    }

    fn get_command(&self) -> &str {
        "echo"
    }
}
