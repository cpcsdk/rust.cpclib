use std::collections::HashSet;
use std::ops::Sub;

use cpclib_cprcli::{build_command, commands::Command};
use cpclib_common::camino::Utf8PathBuf;
use cpclib_cpr::Cpr;

fn main() {
    let cmd = build_command();
    let args = cmd.get_matches();

    let mut cpr = {
        let cpr_fname = args.get_one::<Utf8PathBuf>("INPUT").unwrap();
        Cpr::load(cpr_fname).unwrap()
    };

    let mut cpr2 = args
        .get_one::<Utf8PathBuf>("INPUT2")
        .map(|cpr_fname2| Cpr::load(cpr_fname2).unwrap());

    if let Some(banks) = args.get_many::<i64>("SELECTED_BANKS") {
        let cprs = [&cpr].into_iter().chain(cpr2.as_ref());
        let available = cprs
            .flat_map(|cpr| cpr.banks().iter().map(|b| b.number()))
            .collect::<HashSet<u8>>();
        let to_keep = banks.map(|b| *b as u8).collect::<HashSet<u8>>();

        let missing = to_keep.sub(&available);
        if !missing.is_empty() {
            eprintln!("These banks are not available {missing:?}");
        }

        let to_remove = available.sub(&to_keep);

        for bank in to_remove.into_iter() {
            cpr.remove_bank(bank as _)
                .unwrap_or_else(|| panic!("Bank {bank} not present"));
            cpr2.as_mut().map(|cpr| {
                cpr.remove_bank(bank as _)
                    .unwrap_or_else(|| panic!("Bank {bank} not present"))
            });
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
