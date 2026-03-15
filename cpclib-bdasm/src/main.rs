use clap::Parser;
use cpclib_bdasm::{BdAsmCli, process};
use cpclib_common::event::EventObserver;

fn main() {
    let o = &();
    let cli = match BdAsmCli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind;
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    o.emit_stdout(&e.to_string());
                    return;
                },
                _ => {
                    o.emit_stderr(&e.to_string());
                    std::process::exit(1);
                }
            }
        }
    };
    if let Err(e) = process(&cli, o) {
        o.emit_stderr(&format!("Error: {}", e));
        std::process::exit(1);
    }
}
