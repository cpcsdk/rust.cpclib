use std::process;

use clap::Parser;
use cpclib_bndbuild::pipeline::zik2cpc::{Zik2CpcArgs, run};
use cpclib_common::event::EventObserver;

fn main() {
    let o = &();
    let args = match Zik2CpcArgs::try_parse() {
        Ok(args) => args,
        Err(e) => {
            use clap::error::ErrorKind;
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    o.emit_stdout(&e.to_string());
                    return;
                },
                _ => {
                    o.emit_stderr(&e.to_string());
                    process::exit(1);
                }
            }
        }
    };

    if let Err(e) = run(&args, o) {
        o.emit_stderr(&format!("{e}\n"));
        process::exit(1);
    }
}
