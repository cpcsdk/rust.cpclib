use std::marker::PhantomData;
use std::sync::Arc;

use clap::ArgMatches;
use cpclib_common::clap::{self, Command};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::event::{BndBuilderEvent, BndBuilderObserver, BndBuilderObserverRc, BndBuilderState};
use crate::task::BNDBUILD_CMDS;

pub struct BndBuildRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver + Clone + 'static> Default for BndBuildRunner<E> {
    fn default() -> Self {
        let command = crate::build_args_parser();
        let command = command.no_binary_name(true).after_help(format!(
            "{} {} embedded by {} {}",
            crate::built_info::PKG_NAME,
            crate::built_info::PKG_VERSION,
            built_info::PKG_NAME,
            built_info::PKG_VERSION
        ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver + Clone + 'static> RunnerWithClap for BndBuildRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }

    fn get_matches<S: AsRef<str>>(
        &self,
        itr: &[S],
        e: &dyn EventObserver
    ) -> Result<Option<ArgMatches>, String> {
        let args = match self
            .get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
        {
            Ok(args) => args,
            Err(err)
                if err.kind() == clap::error::ErrorKind::DisplayHelp
                    || err.kind() == clap::error::ErrorKind::DisplayVersion =>
            {
                // Standard clap help/version (shouldn't happen since flags are disabled,
                // but kept as a safety net)
                e.emit_stdout(&err.to_string());
                return Ok(None);
            },
            Err(err) => {
                e.emit_stderr(&err.to_string());
                return Err(String::from("Argument parsing failed"));
            }
        };

        // The bndbuild --help is a custom value-taking arg (for sub-command help).
        // When explicitly passed on the command line, intercept it and emit via observer.
        if args.value_source("version") == Some(clap::parser::ValueSource::CommandLine)
            && args.get_flag("version")
        {
            self.emit_version(e);
            return Ok(None);
        }

        if args.value_source("help") == Some(clap::parser::ValueSource::CommandLine) {
            self.emit_help(e);
            return Ok(None);
        }

        Ok(Some(args))
    }
}

impl<E: EventObserver + Clone + 'static> RunnerWithClapMatches for BndBuildRunner<E> {}

/// Wraps an `EventObserver` and implements `BndBuilderObserver` for nested
/// bndbuild invocations.  All build lifecycle events are converted to formatted
/// text and forwarded through the parent observer so they appear in the TUI
/// (or test-capturing observer) instead of leaking to the raw terminal.
struct ForwardingBndBuilderObserver<E: EventObserver>(E);

impl<E: EventObserver + Clone + 'static> std::fmt::Debug for ForwardingBndBuilderObserver<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForwardingBndBuilderObserver").finish()
    }
}

impl<E: EventObserver + Clone + 'static> EventObserver for ForwardingBndBuilderObserver<E> {
    fn emit_stdout(&self, s: &str) {
        self.0.emit_stdout(s);
    }

    fn emit_stderr(&self, s: &str) {
        self.0.emit_stderr(s);
    }
}

impl<E: EventObserver + Clone + 'static> BndBuilderObserver for ForwardingBndBuilderObserver<E> {
    fn update(&mut self, event: BndBuilderEvent) {
        match event {
            BndBuilderEvent::ChangeState(s) => match s {
                BndBuilderState::ComputeDependencies(p) => {
                    self.0
                        .emit_stdout(&format!("> Compute dependencies for rule `{p}`\n"));
                },
                BndBuilderState::RunTasks => self.0.emit_stdout("> Execute tasks\n"),
                BndBuilderState::Finish => self.0.emit_stdout("> Done.\n"),
            },
            BndBuilderEvent::StartRule { rule, nb, out_of } => {
                self.0
                    .emit_stdout(&format!("[{nb}/{out_of}] Handle {rule}\n"));
            },
            BndBuilderEvent::StartRuleAlias { alias, representative: _, nb, out_of } => {
                self.0
                    .emit_stdout(&format!("[{nb}/{out_of}] Handle {alias}\n"));
            },
            BndBuilderEvent::StopRule(_) => {},
            BndBuilderEvent::FailedRule(rule) => {
                self.0.emit_stderr(&format!("[FAIL] {rule}\n"));
            },
            BndBuilderEvent::StartTask(_r, t) => {
                self.0.emit_stdout(&format!("\t$ {t}\n"));
            },
            BndBuilderEvent::StopTask(_r, _t, d) => {
                self.0.emit_stdout(&format!(
                    "\tElapsed time: {}\n",
                    fancy_duration::FancyDuration(d).truncate(1)
                ));
            },
            BndBuilderEvent::TaskStdout(tgt, _task, txt) => {
                for line in txt.lines() {
                    self.0.emit_stdout(&format!("[{tgt}]\t{line}\n"));
                }
            },
            BndBuilderEvent::TaskStderr(tgt, _task, txt) => {
                for line in txt.lines() {
                    self.0.emit_stderr(&format!("[{tgt}]\t{line}\n"));
                }
            },
            BndBuilderEvent::Stdout(s) => self.0.emit_stdout(s),
            BndBuilderEvent::Stderr(s) => self.0.emit_stderr(s),
        }
    }
}

impl<E: EventObserver + Clone + 'static> Runner for BndBuildRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String> {
        // backup of cwd
        let cwd = std::env::current_dir().unwrap();

        // this will change the cwd
        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        // Forward all nested build output through the parent observer so it
        // reaches the TUI/capturing observer instead of leaking to the terminal.
        // `o.clone()` is cheap when E = Arc<T> (just increments the ref-count).
        let obs = BndBuilderObserverRc::new(ForwardingBndBuilderObserver(o.clone()));
        let res = crate::process_matches_with_observer(&matches, obs);
        // restoration of cwd
        std::env::set_current_dir(cwd).unwrap();

        res.map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        BNDBUILD_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_bndbuild_help_flag_captured() {
        let runner = super::BndBuildRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_bndbuild_version_flag_captured() {
        let runner = super::BndBuildRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_bndbuild_invalid_arg_captured() {
        let runner = super::BndBuildRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
