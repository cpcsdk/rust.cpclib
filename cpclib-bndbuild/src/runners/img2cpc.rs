use cpclib_runner::event::EventObserver;

#[allow(unused_imports)]
use super::{Runner, RunnerWithClap};
use crate::task::IMG2CPC_CMDS;

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner! {
    ImgToCpcRunner,
    cpclib_imgconverter::build_img2cpc_args_parser(),
    IMG2CPC_CMDS[0],
    cpclib_imgconverter::built_info::PKG_NAME,
    cpclib_imgconverter::built_info::PKG_VERSION,
    |matches, command: &clap::Command| cpclib_imgconverter::process_img2cpc(&matches, command.clone())
        .map_err(|e| e.to_string())
}
