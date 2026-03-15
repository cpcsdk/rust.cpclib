use clap::{Arg, ArgAction, Parser};
use cpclib_catalog::cli::CatalogApp;
use cpclib_common::clap::{self, Command, CommandFactory};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::task::{CATALOG_CMDS, DISC_CMDS};

// Using the macro to generate DiscManagerRunner
crate::define_custom_builder_runner! {
    o: simple: DiscManagerRunner,
    cpclib_disc::dsk_manager_build_arg_parser(),
    DISC_CMDS[0],
    cpclib_disc::built_info::PKG_NAME,
    cpclib_disc::built_info::PKG_VERSION,
    |matches, o| cpclib_disc::dsk_manager_handle(&matches, o).map_err(|e| e.to_string())
}

// Note: CatalogRunner needs manual implementation because it uses
// CatalogApp::try_parse_from with special argument chaining
pub struct CatalogRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: std::marker::PhantomData<E>
}

impl<E: EventObserver> Default for CatalogRunner<E> {
    fn default() -> Self {
        let command = cpclib_catalog::cli::CatalogApp::command();
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
                "{} {} embedded by {} {}",
                cpclib_catalog::built_info::PKG_NAME,
                cpclib_catalog::built_info::PKG_VERSION,
                crate::built_info::PKG_NAME,
                crate::built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for CatalogRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for CatalogRunner<E> {}

impl<E: EventObserver> Runner for CatalogRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        // Check for help and version flags first, using get_matches
        let matches = self.get_matches(itr, o)?;
        if matches.is_none() {
            return Ok(());
        }

        // Process the actual command
        let app = CatalogApp::try_parse_from(
            [self.get_command().to_string()]
                .into_iter()
                .chain(itr.iter().map(|s| s.as_ref().to_string()))
        )
        .map_err(|e| e.to_string())?;
        cpclib_catalog::handle_catalog_command(app)
    }

    fn get_command(&self) -> &str {
        CATALOG_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_discmanager_help_flag_captured() {
        let runner = super::DiscManagerRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_discmanager_version_flag_captured() {
        let runner = super::DiscManagerRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_discmanager_invalid_arg_captured() {
        let runner = super::DiscManagerRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }

    #[test]
    fn test_catalog_help_flag_captured() {
        let runner = super::CatalogRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_catalog_version_flag_captured() {
        let runner = super::CatalogRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_catalog_invalid_arg_captured() {
        let runner = super::CatalogRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
