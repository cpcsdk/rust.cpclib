use clap::{Arg, ArgAction, Parser};
use cpclib_catalog::cli::CatalogApp;
use cpclib_common::clap::{self, Command, CommandFactory};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::task::{CATALOG_CMDS, DISC_CMDS};

// Using the macro to generate DiscManagerRunner
crate::define_custom_builder_runner_simple! {
    DiscManagerRunner,
    cpclib_disc::dsk_manager_build_arg_parser(),
    DISC_CMDS[0],
    cpclib_disc::built_info::PKG_NAME,
    cpclib_disc::built_info::PKG_VERSION,
    |matches| cpclib_disc::dsk_manager_handle(&matches).map_err(|e| e.to_string())
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
