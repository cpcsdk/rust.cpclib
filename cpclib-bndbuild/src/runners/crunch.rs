use cpclib_common::event::EventObserver;
use cpclib_crunch::CrunchArgs;
#[allow(unused_imports)]
use cpclib_runner::runner::Runner;
use cpclib_runner::runner::runner::RunnerWithClapDerive;

use crate::task::CRUNCH_CMDS;

// Using the macro to generate all the boilerplate; process receives (cli, observer)
crate::define_clap_derive_runner! {
    o: CrunchRunner,
    CrunchArgs,
    CRUNCH_CMDS[0],
    concat!("cpclib-crunch ", env!("CARGO_PKG_VERSION")),
    cpclib_crunch::process
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_crunch_help_flag_captured() {
        let runner = super::CrunchRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed");
        assert!(!obs.stdout_joined().is_empty(), "help text should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "help should not emit to stderr");
    }

    #[test]
    fn test_crunch_version_flag_captured() {
        let runner = super::CrunchRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--version"], &obs);
        assert!(result.is_ok(), "version should succeed");
        assert!(!obs.stdout_joined().is_empty(), "version string should appear in observer stdout");
        assert!(obs.get_stderr().is_empty(), "version should not emit to stderr");
    }

    #[test]
    fn test_crunch_invalid_arg_captured() {
        let runner = super::CrunchRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--not-a-valid-flag"], &obs);
        assert!(result.is_err(), "invalid argument should fail");
        assert!(!obs.get_stderr().is_empty(), "clap error should be emitted to observer stderr");
        assert!(obs.get_stdout().is_empty(), "no stdout on arg error");
    }
}
