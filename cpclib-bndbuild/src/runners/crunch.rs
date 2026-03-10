use cpclib_common::event::EventObserver;
use cpclib_crunch::CrunchArgs;
use cpclib_runner::runner::runner::RunnerWithClapDerive;
#[allow(unused_imports)]
use cpclib_runner::runner::Runner;

use crate::task::CRUNCH_CMDS;

// Using the macro to generate all the boilerplate
crate::define_clap_derive_runner! {
    CrunchRunner,
    CrunchArgs,
    CRUNCH_CMDS[0],
    concat!("cpclib-crunch ", env!("CARGO_PKG_VERSION")),
    |matches| cpclib_crunch::process(matches)
}
