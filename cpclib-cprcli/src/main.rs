use std::path::PathBuf;

use commands::Command;
use cpclib_common::clap::{self, builder::ValueParser, Arg, ArgAction};
use cpclib_cpr::Cpr;



mod commands;

fn main() {
    let cmd = clap::Command::new("cprcli")
        .about("Command line CPR analysis")
        .arg(
            Arg::new("INFO")
                .help("Show information about the CPR")
                .long("info")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("INPUT")
                .help("The CPR file to read.")
                .long("cpr1")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(ValueParser::path_buf())
        )
        .arg(
            Arg::new("INPUT2")
                .help("The CPR file to compare with.")
                .long("cpr2")
                .required(false)
                .action(ArgAction::Set)
                .value_parser(ValueParser::path_buf())
        )
        ;
    let args = cmd.get_matches();

    let mut cpr = {
        let cpr_fname = args.get_one::<PathBuf>("INPUT").unwrap();
        Cpr::load(cpr_fname).unwrap()
    };

    let mut cpr2 = if let Some(cpr_fname2) = args.get_one::<PathBuf>("INPUT2") {
        Some(Cpr::load(cpr_fname2).unwrap())
    } else {
        None
    };

    let cmd = if args.get_flag("INFO") {
        Command::Info
    } else {
        panic!("No command provided");
    };

    cmd.handle(&mut cpr, cpr2.as_mut());
    
}
