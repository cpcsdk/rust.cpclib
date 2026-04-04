use cpclib_runner::event::EventObserver;

#[allow(unused_imports)]
use super::{Runner, RunnerWithClap};
use crate::task::IMG2CPC_CMDS;

// Using the macro; process receives (matches, command, observer)
crate::define_custom_builder_runner! {
    o: ImgToCpcRunner,
    cpclib_imgconverter::build_img2cpc_args_parser(),
    IMG2CPC_CMDS[0],
    cpclib_imgconverter::built_info::PKG_NAME,
    cpclib_imgconverter::built_info::PKG_VERSION,
    |matches, command: &clap::Command, o| cpclib_imgconverter::process_img2cpc(&matches, command.clone(), o)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_img2cpc_help_flag_captured() {
        let runner = super::ImgToCpcRunner::default();
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
    fn test_img2cpc_version_flag_captured() {
        let runner = super::ImgToCpcRunner::default();
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
    fn test_img2cpc_invalid_arg_captured() {
        let runner = super::ImgToCpcRunner::default();
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
