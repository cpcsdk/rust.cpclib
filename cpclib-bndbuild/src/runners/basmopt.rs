use std::marker::PhantomData;

use clap::{Arg, ArgAction, Command, CommandFactory, FromArgMatches};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::exec::RunnerWithClapMatches;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::BASMOPT_CMDS;

pub struct BasmOptRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for BasmOptRunner<E> {
    fn default() -> Self {
        let command = <cpclib_basmopt::cli::Cli as CommandFactory>::command()
            .name(BASMOPT_CMDS[0])
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
            )
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(ArgAction::SetTrue)
                    .exclusive(true)
            );
        Self {
            command,
            _phantom: PhantomData
        }
    }
}

impl<E: EventObserver + 'static> RunnerWithClap for BasmOptRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver + 'static> RunnerWithClapMatches for BasmOptRunner<E> {}

impl<E: EventObserver + 'static> Runner for BasmOptRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let itr: Vec<&str> = itr.iter().map(|s| s.as_ref()).collect();
        let matches = self.get_matches(&itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        if matches.get_flag("version") {
            o.emit_stdout(&format!("basmopt {}\n", env!("CARGO_PKG_VERSION")));
            return Ok(());
        }

        let cli =
            cpclib_basmopt::cli::Cli::from_arg_matches(&matches).map_err(|e| e.to_string())?;
        let options = cli.options();

        let cpclib_basmopt::AnalyzeOutcome {
            source,
            suggestions,
            assemble_warning
        } = cpclib_basmopt::analyze_file(&cli.source, &options).map_err(|e| e.to_string())?;
        if let Some(warning) = &assemble_warning {
            o.emit_stdout(&format!(
                "{}: warning: could not fully assemble, address-aware suggestions skipped: {warning}\n",
                cli.source
            ));
        }

        if suggestions.is_empty() {
            if !cli.in_place {
                o.emit_stdout(&format!(
                    "{}: no optimization opportunities found\n",
                    cli.source
                ));
            }
            return Ok(());
        }

        if cli.in_place {
            let fixed = cpclib_basmopt::apply_fixes(&source, &suggestions);
            fs_err::write(&cli.source, fixed)
                .map_err(|e| format!("cannot write {}: {e}", cli.source))?;
            o.emit_stdout(&format!(
                "{}: applied {} fix{}\n",
                cli.source,
                suggestions.len(),
                if suggestions.len() == 1 { "" } else { "es" }
            ));
            Ok(())
        }
        else {
            for s in &suggestions {
                let rule = s.rule_name.as_deref().unwrap_or("<unnamed>");
                o.emit_stdout(&format!(
                    "{}:{}:{}: [{rule}] {}\n",
                    cli.source, s.line, s.column, s.message
                ));
            }
            // Same convention as `AsmFmtRunner`'s `--check`: an actionable
            // finding that was not fixed fails the task, so a bndbuild rule
            // that just wants to *report* (not enforce) prefixes the command
            // with `-` for `ignore_error`.
            Err(format!(
                "{} optimization opportunit{} found",
                suggestions.len(),
                if suggestions.len() == 1 { "y" } else { "ies" }
            ))
        }
    }

    fn get_command(&self) -> &str {
        BASMOPT_CMDS[0]
    }
}
