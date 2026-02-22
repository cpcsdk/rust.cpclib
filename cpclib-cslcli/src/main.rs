use std::process;

use clap::Parser;
use cpclib_cslcli::{run, CslCliArgs};

fn main() {
    let args = CslCliArgs::parse();

    if let Err(e) = run(&args) {
        eprintln!("{:?}", e);
        process::exit(1);
    }
}
