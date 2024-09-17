use std::marker::PhantomData;
use std::rc::Rc;

use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_runner::event::EventObserver;

use super::Runner;
use crate::{built_info, expand_glob};

pub struct RmRunner<E: EventObserver> {
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for RmRunner<E> {
    fn default() -> Self {
        Self {
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RmRunner<E> {
    pub fn render_help() -> String {
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
            .render_long_help()
            .to_string()
    }
}
impl<E: EventObserver> Runner for RmRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
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
                std::fs::remove_dir_all(fname)
            }
            else {
                std::fs::remove_file(fname)
            };

            match res {
                Ok(_) => {
                    o.emit_stdout(&format!("\t{} removed", fname /* .display() */));
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
