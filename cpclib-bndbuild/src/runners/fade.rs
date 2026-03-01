use cpclib_common::event::EventObserver;
use cpclib_imgconverter::{fade_build_args, fade_handle_matches};
use cpclib_runner::runner::runner::RunnerWithClapMatches;
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::FADE_CMDS;

pub const FADE_CMD: &str = "fade";

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner_simple! {
    FadeRunner,
    fade_build_args(),
    FADE_CMDS[0],
    cpclib_imgconverter::built_info::PKG_NAME,
    cpclib_imgconverter::built_info::PKG_VERSION,
    |matches| fade_handle_matches(&matches).map_err(|e| e.to_string())
}
