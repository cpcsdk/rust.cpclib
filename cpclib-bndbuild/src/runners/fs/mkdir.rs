use std::marker::PhantomData;

use clap::Command;
use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;

use crate::runners::{Runner, RunnerWithClap};
use crate::task::MKDIR_CMDS;
use crate::{built_info, expand_glob};

pub struct MkdirRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for MkdirRunner<E> {
    fn default() -> Self {
        Self {
            command: clap::Command::new("mkdir")
                .before_help("Create directories.")
                .disable_help_flag(true)
                .after_help(format!(
                    "Inner command of {} {}",
                    built_info::PKG_NAME,
                    built_info::PKG_VERSION
                ))
                .arg(
                    Arg::new("parents")
                        .long("parents")
                        .short('p')
                        .action(ArgAction::SetTrue)
                        .help("Set to specify if missing parent directories must be created")
                )
                .arg(
                    Arg::new("arguments")
                        .action(ArgAction::Append)
                        .help("Folders to create.")
                ),
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> MkdirRunner<E> {
    pub fn render_help() -> String {
        Self::default().command.render_long_help().to_string()
    }
}

impl<E: EventObserver> RunnerWithClap for MkdirRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> Runner for MkdirRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let mut errors = String::new();

        let matches = {
            let mut itr = itr.iter().map(|s| s.as_ref()).collect_vec();
            itr.insert(0, "mkdir");
            self.get_matches(&itr)?
        };

        let parents = matches.get_flag("parents");

        for fname in matches
            .get_many::<String>("arguments")
            .unwrap()
            .map(|s| s.as_ref())
            .flat_map(expand_glob)
        {
            let fname = Utf8Path::new(&fname);

            if fname.exists() {
                errors.push_str(&format!("{} already exists\n", fname,))
            }
            else {
                let res = if parents {
                    std::fs::create_dir_all(fname)
                }
                else {
                    std::fs::create_dir(fname)
                };

                match res {
                    Ok(_) => {
                        o.emit_stdout(&format!("\t{} created", fname));
                    },
                    Err(e) => errors.push_str(&format!("Unable to create {}. {}.\n", fname, e))
                };
            }
        }

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    fn get_command(&self) -> &str {
        MKDIR_CMDS[0]
    }
}
