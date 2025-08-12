use std::process::exit;

use clap::Parser;
use cpclib_runner::emucontrol::{EmuCli, handle_arguments};

fn main() {
    let cli = EmuCli::parse();
    let res = handle_arguments(cli, &());

    match res {
        Ok(_) => println!("No error occurred."),
        Err(e) => {
            eprintln!("An error occurred. {e}");
            exit(-1);
        }
    }
}
