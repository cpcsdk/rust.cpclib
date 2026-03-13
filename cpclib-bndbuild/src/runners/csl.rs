use cpclib_cslcli::CslCliArgs;
use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::Runner;
use cpclib_runner::runner::runner::RunnerWithClapDerive;

use crate::task::CSL_CMDS;

// Using the macro to generate all the boilerplate
crate::define_clap_derive_runner! {
    CslRunner,
    CslCliArgs,
    CSL_CMDS[0],
    concat!("cpclib-cslcli ", env!("CARGO_PKG_VERSION")),
    |cli| cpclib_cslcli::run(&cli).map_err(|e| e.to_string())
}
