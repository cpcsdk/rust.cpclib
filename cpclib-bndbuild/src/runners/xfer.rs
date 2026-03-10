use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};

use crate::task::XFER_CMDS;

// Using the macro to generate all the boilerplate
crate::define_custom_builder_runner_simple! {
    XferRunner,
    cpclib_xfertool::build_args_parser(),
    XFER_CMDS[0],
    cpclib_xfertool::built_info::PKG_NAME,
    cpclib_xfertool::built_info::PKG_VERSION,
    |matches| cpclib_xfertool::process(&matches).map_err(|e| e.to_string())
}
