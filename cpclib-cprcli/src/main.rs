use std::{collections::HashSet, ops::Sub, path::PathBuf};

use commands::Command;
use cpclib_common::{clap::{self, builder::ValueParser, value_parser, Arg, ArgAction}, itertools::Itertools};
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
            Arg::new("SELECTED_BANKS")
                .help("Select banks of interest")
                .long("bank")
                .action(ArgAction::Append)
                .value_parser(0..32)
                .value_delimiter(',')
        )
        .arg(
            Arg::new("DUMP")
                .help("Get the memory of interest")
                .long("dump")
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

    if let Some(banks) = args.get_many::<i64>("SELECTED_BANKS") {
        let cprs = [&cpr].into_iter().chain(cpr2.as_ref());
        let available = cprs.map(|cpr| cpr.banks().iter().map(|b| b.number()))
                                                                        .flatten()
                                                                        .collect::<HashSet<u8>>();
        let to_keep = banks.map(|b| *b as u8).collect::<HashSet<u8>>();

        let missing = to_keep.sub(&available);
        if !missing.is_empty() {
            eprintln!("These banks are not available {:?}", missing);
        }

        let to_remove = available.sub(&to_keep);


        for bank in to_remove.into_iter() {
            cpr.remove_bank(bank as _).expect("Bank {bank} not present");
            cpr2.as_mut().map(|cpr| cpr.remove_bank(bank as _).expect("Bank {bank} not present"));
        }
    }

    let cmd = if args.get_flag("INFO") {
        Command::Info
    } 
    else if args.get_flag("DUMP") {
        Command::Dump
    }
    else {
        panic!("No command provided");
    };

    cmd.handle(&mut cpr, cpr2.as_mut());
    
}
