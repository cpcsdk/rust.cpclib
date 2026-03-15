use std::marker::PhantomData;

use clap::ArgMatches;
use cpclib_common::clap::{self, Command};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::built_info;
use crate::event::BndBuilderObserverRc;
use crate::task::BNDBUILD_CMDS;

pub struct BndBuildRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for BndBuildRunner<E> {
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

impl<E: EventObserver> RunnerWithClap for BndBuildRunner<E> {
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

impl<E: EventObserver> RunnerWithClapMatches for BndBuildRunner<E> {}

impl<E: EventObserver> Runner for BndBuildRunner<E> {
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

        // BUG here we skip the observer. It is necessary to find a way to use it properly
        let o = BndBuilderObserverRc::new_default();
        let res = crate::process_matches_with_observer(&matches, o);
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
