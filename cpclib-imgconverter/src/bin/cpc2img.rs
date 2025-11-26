use cpclib_imgconverter::{self, build_cpc2img_args_parser};

fn main() -> Result<(), String> {
    let cmd = build_cpc2img_args_parser();

    let matches = cmd.clone().get_matches();

    cpclib_imgconverter::process_cpc2img(&matches, cmd.clone()).map_err(|e| e.to_string())
}
