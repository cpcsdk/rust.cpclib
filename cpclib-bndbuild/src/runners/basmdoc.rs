pub const BASMDOC_CMD: &str = "basmdoc";

#[cfg(feature = "basmdoc-generator")]
use cpclib_common::event::EventObserver;
#[cfg(feature = "basmdoc-generator")]
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

// Using the macro to generate all the boilerplate. Gated behind
// `basmdoc-generator` since it needs `cpclib_basmdoc::cmdline`, which only
// exists when `cpclib-basmdoc` itself is built with its own `generator`
// feature - kept optional so a consumer of `cpclib-bndbuild` that doesn't
// want basmdoc's heavy rendering/CLI dependency graph (pandoc/ureq/
// minijinja/...) transitively pulled in isn't forced to (Cargo unifies
// features per-crate across a build, so an unconditional dependency here
// would leak into every other consumer of `cpclib-basmdoc` in the same
// build graph - see `cpclib-lsp`, which depends on both `cpclib-bndbuild`
// and `cpclib-basmdoc` directly and wants the latter to stay lightweight).
#[cfg(feature = "basmdoc-generator")]
crate::define_custom_builder_runner! {
    BasmDocRunner,
    cpclib_basmdoc::cmdline::build_args_parser(),
    BASMDOC_CMD,
    "cpclib-basmdoc",
    env!("CARGO_PKG_VERSION"),
    |matches, command| cpclib_basmdoc::cmdline::handle_matches(&matches, command)
        .map_err(|e| e.to_string())
}

#[cfg(all(test, feature = "basmdoc-generator"))]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_basmdoc_help_flag_captured() {
        let runner = super::BasmDocRunner::default();
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
    fn test_basmdoc_version_flag_captured() {
        let runner = super::BasmDocRunner::default();
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
    fn test_basmdoc_invalid_arg_captured() {
        let runner = super::BasmDocRunner::default();
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
