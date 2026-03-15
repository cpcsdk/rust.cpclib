pub const BASMDOC_CMD: &str = "basmdoc";

use cpclib_common::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner! {
    BasmDocRunner,
    cpclib_basmdoc::cmdline::build_args_parser(),
    BASMDOC_CMD,
    "cpclib-basmdoc",
    env!("CARGO_PKG_VERSION"),
    |matches, command| cpclib_basmdoc::cmdline::handle_matches(&matches, command)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_basmdoc_help_flag_captured() {
        let runner = super::BasmDocRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_basmdoc_version_flag_captured() {
        let runner = super::BasmDocRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_basmdoc_invalid_arg_captured() {
        let runner = super::BasmDocRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
