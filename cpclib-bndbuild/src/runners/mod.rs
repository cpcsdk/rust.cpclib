/// Manage the standard tasks
use cpclib_basm;
use cpclib_common::clap::{self, Arg, ArgAction, ArgMatches, Command};
use cpclib_common::itertools::Itertools;
use cpclib_disc::dsk_manager_build_arg_parser;
use glob::glob;
use shlex::split;

use crate::{built_info, expand_glob};

pub mod basm;
pub mod dsk;
pub mod echo;
pub mod r#extern;
pub mod imgconverter;
pub mod rm;
pub mod xfer;

/// Get all args (split string as done in shell and apply glob matching)
fn get_all_args(arguments: &str) -> Vec<String> {
    let init_args = split(arguments).unwrap_or_default();
    let mut res = Vec::new();
    for p in init_args {
        match glob(&p) {
            Ok(entries) => {
                let mut added = 0;
                for entry in entries {
                    match entry {
                        Ok(p) => res.push(p.display().to_string()),
                        Err(e) => res.push(e.path().display().to_string())
                    }
                    added += 1;
                }
                if added == 0 {
                    res.push(p);
                }
            }
            Err(_) => res.push(p)
        }
    }
    res
}

pub trait Runner {
    /// Run the task and return true if successfull
    fn run(&self, arguments: &str) -> Result<(), String> {
        println!("\t$ {} {}", self.get_command(), arguments);
        let args = get_all_args(&arguments.replace(r"\", r"\\"));
        self.inner_run(&args)
    }

    /// Implement the command specific action
    fn inner_run(&self, itr: &[String]) -> Result<(), String>;

    fn get_command(&self) -> &str;
}

pub trait RunnerWithClap: Runner {
    fn get_clap_command(&self) -> &Command;

    fn get_matches(&self, itr: &[String]) -> Result<ArgMatches, String> {
        self.get_clap_command()
            .clone()
            .try_get_matches_from(itr)
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