pub const BASMDOC_CMD: &str = "basmdoc";

use cpclib_common::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;
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
