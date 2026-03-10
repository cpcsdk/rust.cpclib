use cpclib_disc::hideur::{hideur_build_arg_parser, hideur_handle};
use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::HIDEUR_CMDS;

pub const HIDEUR_CMD: &str = "hideur";

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner! {
    simple: HideurRunner,
    hideur_build_arg_parser(),
    HIDEUR_CMDS[0],
    cpclib_disc::built_info::PKG_NAME,
    cpclib_disc::built_info::PKG_VERSION,
    |matches| hideur_handle(&matches).map_err(|e| e.to_string())
}
