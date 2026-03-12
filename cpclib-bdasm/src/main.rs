use clap::Parser;
use cpclib_bdasm::{BdAsmCli, process};

fn main() {
    let cli = BdAsmCli::parse();
    if let Err(e) = process(&cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
