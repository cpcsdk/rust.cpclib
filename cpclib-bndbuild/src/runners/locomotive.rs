use clap::{Arg, ArgAction, CommandFactory, Parser};
use cpclib_common::clap::Command;
use cpclib_locomotive::Cli;
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::task::LOCOMOTIVE_CMDS;

// Note: LocomotiveRunner needs manual implementation because it uses
// Cli::try_parse_from with special argument chaining, which the macro doesn't support
pub struct LocomotiveRunner<E: EventObserver> {
    command: Command,
    _phantom: std::marker::PhantomData<E>
}

impl<E: EventObserver> Default for LocomotiveRunner<E> {
    fn default() -> Self {
        let command = Cli::command();
        let command = command
            .disable_help_flag(true)
            .disable_version_flag(true)
            .subcommand_required(false) // Allow --help without subcommand
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
            )
            .no_binary_name(true)
            .after_help(format!(
                "cpclib-locomotive embedded by {} {}",
                crate::built_info::PKG_NAME,
                crate::built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for LocomotiveRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for LocomotiveRunner<E> {}

impl<E: EventObserver> Runner for LocomotiveRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        // Check for help and version flags first, using get_matches
        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }

        let cli = Cli::try_parse_from(
            [self.get_command().to_string()]
                .into_iter()
                .chain(itr.iter().map(|s| s.as_ref().to_string()))
        )
        .map_err(|e| e.to_string())?;

        cpclib_locomotive::handle_locomotive_arguments(cli).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        LOCOMOTIVE_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_locomotive_help_flag_captured() {
        let runner = super::LocomotiveRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(
            !obs.stdout_joined().is_empty(),
            "help text should appear in observer stdout"
        );
        assert!(
            obs.get_stderr().is_empty(),
            "help should not emit to stderr"
        );
    }

    #[test]
    fn test_locomotive_version_flag_captured() {
        let runner = super::LocomotiveRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(
            !obs.stdout_joined().is_empty(),
            "version string should appear in observer stdout"
        );
        assert!(
            obs.get_stderr().is_empty(),
            "version should not emit to stderr"
        );
    }

    #[test]
    fn test_locomotive_invalid_arg_captured() {
        let runner = super::LocomotiveRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(
            !obs.get_stderr().is_empty(),
            "clap error should be emitted to observer stderr"
        );
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
