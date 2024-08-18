use clap::{ArgMatches, Command};
use cpclib_common::itertools::Itertools;

use crate::runner::arguments::get_all_args;

pub trait Runner {
    /// Run the task and return true if successfull
    fn run(&self, arguments: &str) -> Result<(), String> {
        println!("\t$ {} {}", self.get_command(), arguments);
        let args = get_all_args(arguments)?;
        self.inner_run(&args)
    }

    /// Implement the command specific action
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String>;

    fn get_command(&self) -> &str;
}

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

#[derive(Default)]
pub struct ExternRunner {}
impl ExternRunner {}
impl Runner for ExternRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();

        // WARNING
        // Deactivated because if makes fail normal progam on Linux
        // however, it was maybe mandatory for Windows
        // let app = std::fs::canonicalize(&itr[0])
        //     .map_err(|e| format!("Wrong executable {}.{}", &itr[0], e.to_string()))?;
        let app = &itr[0];

        let cwd = std::env::current_dir()
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;
        let cwd = std::fs::canonicalize(cwd)
            .map_err(|e| format!("Unable to get the current working directory {}.", e))?;

        let mut cmd = std::process::Command::new(app);
        cmd.current_dir(cwd);
        for arg in &itr[1..] {
            cmd.arg(arg);
        }
        let mut handle = cmd
            .spawn()
            .map_err(|e| format!("Error while launching {}. {}", &itr[0], e))?;

        let status = handle
            .wait()
            .map_err(|e| format!("Error while executing {}. {}", &itr[0], e))?;

        if !status.success() {
            return Err("Error while launching the command.".to_owned());
        }
        Ok(())
    }

    fn get_command(&self) -> &str {
        "external"
    }
}
