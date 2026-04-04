use cpclib_common::event::EventObserver;
use cpclib_imgconverter::{fade_build_args, fade_handle_matches};
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::FADE_CMDS;

pub const FADE_CMD: &str = "fade";

// Using the macro; process receives (matches, observer)
crate::define_custom_builder_runner! {
    o: simple: FadeRunner,
    fade_build_args(),
    FADE_CMDS[0],
    cpclib_imgconverter::built_info::PKG_NAME,
    cpclib_imgconverter::built_info::PKG_VERSION,
    |matches, o| fade_handle_matches(&matches, o).map_err(|e| e.to_string())
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_fade_help_flag_captured() {
        let runner = super::FadeRunner::default();
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
    fn test_fade_version_flag_captured() {
        let runner = super::FadeRunner::default();
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
    fn test_fade_invalid_arg_captured() {
        let runner = super::FadeRunner::default();
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
