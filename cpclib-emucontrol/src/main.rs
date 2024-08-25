use std::process::exit;
use std::time::Duration;

use clap::Parser;
use cpclib_runner::emucontrol::{handle_arguments, EmuCli};

fn main() {
    let cli = EmuCli::parse();
    let res = handle_arguments(cli);

    match res {
        Ok(_) => println!("No error occurred."),
        Err(e) => {
            eprintln!("An error occurred. {}", e);
            exit(-1);
        }
    }
}
