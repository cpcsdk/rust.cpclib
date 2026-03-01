use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapDerive;
use cpclib_runner::runner::{Runner, RunnerWithClap};
use hxcfe_cli::HxcfeCli;

use crate::task::HXCFE_CMDS;

// Using the macro to generate all the boilerplate
crate::define_clap_derive_runner! {
    HxcfeRunner,
    HxcfeCli,
    HXCFE_CMDS[0],
    concat!("hxcfe_cli ", env!("CARGO_PKG_VERSION")),
    |cli| hxcfe_cli::run(&cli).map_err(|e| e.to_string())
}
