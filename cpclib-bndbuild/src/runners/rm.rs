use cpclib_common::clap::{self, Arg, ArgAction};

use super::Runner;
use crate::{built_info, expand_glob};

#[derive(Default)]
pub struct RmRunner {}

impl RmRunner {
    pub fn print_help(&self) {
        clap::Command::new("rm")
            .before_help("Delete files.")
            .disable_help_flag(true)
            .after_help(format!(
                "Inner command of {} {}",
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            .arg(
                Arg::new("arguments")
                    .action(ArgAction::Append)
                    .help("Files to delete.")
            )
            .print_long_help()
            .unwrap();
    }
}
impl Runner for RmRunner {
    fn inner_run(&self, itr: &[String]) -> Result<(), String> {
        let mut errors = String::new();

        for fname in itr
            .into_iter()
            .map(|s| s.as_str())
            //    .map(|s| glob(s).unwrap())
            .map(expand_glob)
            .flatten()
        //    .map(|e| dbg!(e.unwrap()))
        {
            match std::fs::remove_file(&fname) {
                Ok(_) => {
                    println!("\t{} removed", fname /* .display() */);
                }
                Err(e) => {
                    errors.push_str(&format!(
                        "Unable to remove {}:{}.\n",
                        fname, // .display()
                        e.to_string()
                    ))
                }
            };
        }

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    fn get_command(&self) -> &str {
        "rm"
    }
}
