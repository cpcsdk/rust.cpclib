use cpclib_common::clap::{ArgMatches, Command};
use glob::glob;
use shlex::split;

pub mod basm;
pub mod bndbuild;
pub mod cp;
pub mod disc;
pub mod echo;
pub mod emulator;
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
            },
            Err(_) => res.push(p)
        }
    }
    res
}

pub trait Runner {
    /// Run the task and return true if successfull
    fn run(&self, arguments: &str) -> Result<(), String> {
        println!("\t$ {} {}", self.get_command(), arguments);
        let args = get_all_args(
            &arguments
                .replace(r"\", r"\\")
                .replace("\"", "\\\"")
            );
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
