use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::Runner;
use cpclib_runner::runner::exec::RunnerWithClapDerive;

use crate::pipeline::zik2cpc::Zik2CpcArgs;
use crate::task::ZIK2CPC_CMDS;

// Using the macro to generate all the boilerplate
crate::define_clap_derive_runner! {
    o:
    Zik2CpcRunner,
    Zik2CpcArgs,
    ZIK2CPC_CMDS[0],
    concat!("zik2cpc ", env!("CARGO_PKG_VERSION")),
    |cli, o| crate::pipeline::zik2cpc::run(&cli, o)
}

#[cfg(test)]
mod test {
    use cpclib_common::event::CapturingObserver;
    use cpclib_runner::runner::Runner;

    #[test]
    fn test_zik2cpc_help_flag_captured() {
        let runner = super::Zik2CpcRunner::default();
        let obs = CapturingObserver::new();
        let result = runner.inner_run(&["--help"], &obs);
        assert!(result.is_ok(), "help should succeed: {result:?}");
        assert!(!obs.stdout_joined().is_empty());
        assert!(obs.get_stderr().is_empty());
    }

    #[test]
    fn test_zik2cpc_players_of_a_missing_song() {
        let runner = super::Zik2CpcRunner::default();
        let obs = CapturingObserver::new();
        assert!(
            runner
                .inner_run(&["convert", "/nonexistent.aks", "/tmp/x.akg", "--to", "akg"], &obs)
                .is_err()
        );
    }

    #[test]
    fn test_zik2cpc_invalid_arg() {
        let runner = super::Zik2CpcRunner::default();
        let obs = CapturingObserver::new();
        assert!(runner.inner_run(&["--nope"], &obs).is_err());
    }
}
