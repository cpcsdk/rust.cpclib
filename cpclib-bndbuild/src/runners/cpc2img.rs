use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::runner::RunnerWithClapMatches;

use super::{Runner, RunnerWithClap};
use crate::task::CPC2IMG_CMDS;

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner! {
    CpcToImgRunner,
    cpclib_imgconverter::build_cpc2img_args_parser(),
    CPC2IMG_CMDS[0],
    cpclib_imgconverter::built_info::PKG_NAME,
    cpclib_imgconverter::built_info::PKG_VERSION,
    |matches, command: &clap::Command| cpclib_imgconverter::process_cpc2img(&matches, command.clone())
        .map_err(|e| e.to_string())
}
