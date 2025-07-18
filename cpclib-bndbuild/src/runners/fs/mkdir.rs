use std::marker::PhantomData;

use clap::Command;
use cpclib_common::camino::Utf8Path;
use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_common::itertools::Itertools;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

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
                    Arg::new("ignore")
                        .short('i')
                        .long("ignore")
                        .action(ArgAction::SetTrue)
                        .help("Set to specify we ignore existing folders")
                )
                .arg(
                    Arg::new("arguments")
                        .action(ArgAction::Append)
                        .help("Folders to create.")
                )
                .arg(
                    Arg::new("help")
                        .long("help")
                        .short('h')
                        .action(ArgAction::SetTrue)
                        .exclusive(true) // does not seem to work
                )
                .arg(
                    Arg::new("version")
                        .long("version")
                        .short('V')
                        .help("Print version")
                        .action(ArgAction::SetTrue)
                        .exclusive(true) // does not seem to work
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

impl<E: EventObserver> RunnerWithClapMatches for MkdirRunner<E> {}

impl<E: EventObserver> Runner for MkdirRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let mut errors = String::new();

        let matches = {
            let mut itr = itr.iter().map(|s| s.as_ref()).collect_vec();
            itr.insert(0, "mkdir");
            self.get_matches(&itr, o)?
        };

        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        let parents = matches.get_flag("parents");

        for fname in matches
            .get_many::<String>("arguments")
            .unwrap()
            .map(|s| s.as_ref())
            .flat_map(expand_glob)
        {
            let fname = Utf8Path::new(&fname);

            if fname.exists() {
                if !matches.get_flag("ignore") {
                    errors.push_str(&format!(
                        "{fname} already exists. Use --ignore to not crash.\n",
                    ));
                }
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
                        o.emit_stdout(&format!("\t{fname} created"));
                    },
                    Err(e) => errors.push_str(&format!("Unable to create {fname}. {e}.\n"))
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
