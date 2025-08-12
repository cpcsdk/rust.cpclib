use cpclib_common::clap::Parser;
use cpclib_crunch::CrunchArgs;

fn main() -> Result<(), String> {
    let args = CrunchArgs::parse();
    cpclib_crunch::process(args)
}
