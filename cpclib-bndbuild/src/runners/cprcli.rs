use std::collections::HashSet;
use std::marker::PhantomData;
use std::ops::Sub;

use cpclib_common::camino::Utf8PathBuf;
use cpclib_cprcli::commands::Command;
use cpclib_cpr::Cpr;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::built_info;
use crate::task::CPR_CMDS;

pub const CPRCLI_CMD: &str = "CPRCLI";

pub struct CprCliRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for CprCliRunner<E> {
    fn default() -> Self {
        let command = cpclib_cprcli::build_command();
        let command = command
            .name("cpr")
            .no_binary_name(true)
            .after_help(format!(
                "cpr {} embedded by {} {}",
                env!("CARGO_PKG_VERSION"),
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for CprCliRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for CprCliRunner<E> {}

impl<E: EventObserver> Runner for CprCliRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr, o)?;

        if matches.is_none() {
            return Ok(());
        }
        let args = matches.unwrap();

        // Load the main CPR file
        let mut cpr = {
            let cpr_fname = args
                .get_one::<Utf8PathBuf>("INPUT")
                .ok_or_else(|| "INPUT not provided".to_string())?;
            Cpr::load(cpr_fname).map_err(|e| format!("Failed to load CPR: {e:?}"))?
        };

        // Load the optional second CPR file
        let mut cpr2 = args
            .get_one::<Utf8PathBuf>("INPUT2")
            .map(|cpr_fname2| Cpr::load(cpr_fname2))
            .transpose()
            .map_err(|e| format!("Failed to load second CPR: {e:?}"))?;

        // Handle bank selection if specified
        if let Some(banks) = args.get_many::<i64>("SELECTED_BANKS") {
            let cprs = [&cpr].into_iter().chain(cpr2.as_ref());
            let available = cprs
                .flat_map(|cpr| cpr.banks().iter().map(|b| b.number()))
                .collect::<HashSet<u8>>();
            let to_keep = banks.map(|b| *b as u8).collect::<HashSet<u8>>();

            let missing = to_keep.sub(&available);
            if !missing.is_empty() {
                eprintln!("These banks are not available {missing:?}");
            }

            let to_remove = available.sub(&to_keep);

            for bank in to_remove.into_iter() {
                cpr.remove_bank(bank as _)
                    .unwrap_or_else(|| panic!("Bank {bank} not present"));
                cpr2.as_mut().map(|cpr| {
                    cpr.remove_bank(bank as _)
                        .unwrap_or_else(|| panic!("Bank {bank} not present"))
                });
            }
        }

        // Determine which command to execute
        let cmd = if args.get_flag("INFO") {
            Command::Info
        }
        else if args.get_flag("DUMP") {
            Command::Dump
        }
        else {
            return Err("No command provided".to_string());
        };

        // Execute the command
        cmd.handle(&mut cpr, cpr2.as_mut());

        Ok(())
    }

    fn get_command(&self) -> &str {
        CPR_CMDS[0]
    }
}
