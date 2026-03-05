pub const SNAPSHOT_CMD: &str = "SNAPSHOT";

use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;
use cpclib_runner::runner::{Runner, RunnerWithClap};
use cpclib_sna;

use crate::task::SNA_CMDS;

// Note: SnapshotRunner needs manual implementation because cpclib_sna::process
// requires passing the EventObserver, which the macro doesn't support
pub struct SnapshotRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: std::marker::PhantomData<E>
}

impl<E: EventObserver> Default for SnapshotRunner<E> {
    fn default() -> Self {
        let command = cpclib_sna::build_arg_parser();
        let command = command
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                clap::Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(clap::ArgAction::SetTrue)
                    .exclusive(true)
            )
            .arg(
                clap::Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(clap::ArgAction::SetTrue)
                    .exclusive(true)
            )
            .after_help(format!(
                "{} {} embedded by {} {}",
                cpclib_sna::built_info::PKG_NAME,
                cpclib_sna::built_info::PKG_VERSION,
                crate::built_info::PKG_NAME,
                crate::built_info::PKG_VERSION
            ));
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for SnapshotRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapMatches for SnapshotRunner<E> {}

impl<E: EventObserver> Runner for SnapshotRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let matches = self.get_matches(itr, o)?;

        if matches.is_none() {
            return Ok(());
        }
        let matches = matches.unwrap();

        cpclib_sna::process(&matches, o).map_err(|e| format!("{e:?}"))
    }

    fn get_command(&self) -> &str {
        SNA_CMDS[0]
    }
}
