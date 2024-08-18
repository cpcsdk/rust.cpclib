use cpclib_common::{camino::Utf8Path, clap::{self, Arg, ArgAction}};

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
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let mut errors = String::new();

        for fname in itr
            .iter()
            .map(|s| s.as_ref())
            //    .map(|s| glob(s).unwrap())
            .flat_map(expand_glob)
        //    .map(|e| dbg!(e.unwrap()))
        {

            let fname = Utf8Path::new(&fname);
            let res = if fname.is_dir() {
                std::fs::remove_dir_all(&fname)
            } else {
                std::fs::remove_file(&fname)
            };

            match res {
                Ok(_) => {
                    println!("\t{} removed", fname /* .display() */);
                },
                Err(e) => {
                    errors.push_str(&format!(
                        "Unable to remove {}:{}.\n",
                        fname, // .display()
                        e
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
