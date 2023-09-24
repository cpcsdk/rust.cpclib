#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    warnings
)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::identity_op)]

use std::path::Path;
use std::str::FromStr;

use cpclib_common::clap::{Arg, ArgAction, Command};
use cpclib_sna::{cli, Snapshot, SnapshotFlag};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Convert a string to its unsigned 32 bits representation (to access to extra memory)
#[must_use]
pub fn string_to_nb(source: &str) -> u32 {
    let error = format!("Unable to read the value: {source}");
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    }
    else if source.starts_with('%') {
        u32::from_str_radix(&source[1..], 2).expect(&error)
    }
    else {
        source.parse::<u32>().expect(&error)
    }
}

fn main() {
    eprintln!("[WARNING] This is still a draft version that implement still few functionnalities");

    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );
    let matches = Command::new("createSnapshot")
                          .version(built_info::PKG_VERSION)
                          .disable_version_flag(true)
                          .author("Krusty/Benediction")
                          .about("Amstrad CPC snapshot manipulation")
                          .before_help(desc_before)
                          .after_help("This tool tries to be similar than Ramlaid's one")
                          .arg(Arg::new("info")
                               .help("Display informations on the loaded snapshot")
                               .long("info")
                               .requires("inSnapshot")
                               .action(ArgAction::SetTrue)
                           )
                           .arg(Arg::new("cli")
                               .help("Run the CLI interface for an interactive manipulation of snapshot")
                               .long("cli")
                               .requires("inSnapshot")
                               .action(ArgAction::SetTrue)
                           )
                          .arg(Arg::new("debug")
                            .help("Display debugging information while manipulating the snapshot")
                            .long("debug")
                            .action(ArgAction::SetTrue)
                          )
                          .arg(Arg::new("OUTPUT")
                               .help("Sets the output file to generate")
                               .conflicts_with("flags")
                               .conflicts_with("cli")
                               .conflicts_with("info")
                               .conflicts_with("getToken")
                               .last(true)
                               .required(true))
                          .arg(Arg::new("inSnapshot")
                               .short('i')
                               .long("inSnapshot")
                               .value_name("INFILE")
                               .number_of_values(1)
                               .help("Load <INFILE> snapshot file")
                               )
                          .arg(Arg::new("load")
                               .short('l')
                               .long("load")
                               .action(ArgAction::Append)
                               .number_of_values(2)
                               .help("Specify a file to include. -l fname address"))
                          .arg(Arg::new("getToken")
                               .short('g')
                               .long("getToken")
                               .action(ArgAction::Append)
                               .number_of_values(1)
                               .help("Display the value of a snapshot token")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::new("setToken")
                               .short('s')
                               .long("setToken")
                               .action(ArgAction::Append)
                               .number_of_values(2)
                               .help("Set snapshot token <$1> to value <$2>\nUse <$1>:<val> to set array value\n\t\tex '-s CRTC_REG:6 20' : Set CRTC register 6 to 20"))
                          .arg(Arg::new("putData")
                               .short('p')
                               .long("putData")
                               .action(ArgAction::Append)
                               .number_of_values(2)
                               .help("Put <$2> byte at <$1> address in snapshot memory")

                            )
                          .arg(Arg::new("version")
                                .short('v')
                                .long("version")
                                .number_of_values(1)
                                .value_parser(["1", "2", "3"])
                                .help("Version of the saved snapshot.")
                                .default_value("3")
                           )
                          .arg(Arg::new("flags")
                                .help("List the flags and exit")
                               .long("flags")
                               .action(ArgAction::SetTrue)
                        )
                          .get_matches();

    // Display all tokens
    if matches.get_flag("flags") {
        for flag in SnapshotFlag::enumerate().iter() {
            println!(
                "{:?} / {:?} bytes.{}",
                flag,
                flag.elem_size(),
                flag.comment()
            );
        }
        return;
    }

    // Load a snapshot or generate an empty one
    let mut sna = if matches.contains_id("inSnapshot") {
        let fname = matches.get_one::<String>("inSnapshot").unwrap();
        let path = Path::new(&fname);
        Snapshot::load(path).expect("Error while loading the snapshot")
    }
    else {
        Snapshot::default()
    };

    // Activate the debug mode
    sna.debug = matches.contains_id("debug");

    if matches.get_flag("info") {
        sna.print_info();
        return;
    }

    if matches.get_flag("cli") {
        let fname = matches.get_one::<String>("inSnapshot").unwrap();
        cli::cli(fname, sna);
        return;
    }

    // Manage the files insertion
    if matches.contains_id("load") {
        let loads = matches
            .get_many::<String>("load")
            .unwrap()
            .collect::<Vec<_>>();
        for i in 0..(loads.len() / 2) {
            let fname = loads[i * 2 + 0];
            let place = loads[i * 2 + 1];

            let address = {
                if place.starts_with("0x") {
                    u32::from_str_radix(&place[2..], 16).unwrap()
                }
                else if place.starts_with('0') {
                    u32::from_str_radix(&place[1..], 8).unwrap()
                }
                else {
                    place.parse::<u32>().unwrap()
                }
            };
            sna.add_file(fname, address as usize)
                .expect("Unable to add file");
        }
    }

    // Patch memory
    if matches.contains_id("putData") {
        let data = matches
            .get_many::<String>("putData")
            .unwrap()
            .collect::<Vec<_>>();

        for i in 0..(data.len() / 2) {
            let address = string_to_nb(data[i * 2 + 0]);
            let value = string_to_nb(data[i * 2 + 1]);
            assert!(value < 0x100);

            sna.set_byte(address, value as u8);
        }
    }

    // Read the tokens
    if matches.contains_id("getToken") {
        for token in matches.get_many::<String>("getToken").unwrap() {
            let token = SnapshotFlag::from_str(token).unwrap();
            println!("{:?} => {}", &token, sna.get_value(&token));
        }
        return;
    }

    // Set the tokens
    if matches.contains_id("setToken") {
        let loads = matches
            .get_many::<String>("setToken")
            .unwrap()
            .collect::<Vec<_>>();
        for i in 0..(loads.len() / 2) {
            // Read the parameters from the command line
            let token = dbg!(loads[i * 2 + 0]);
            let token = SnapshotFlag::from_str(token).unwrap();

            let value = {
                let source = loads[i * 2 + 1];
                string_to_nb(source)
            };

            // Get the token
            sna.set_value(token, value as u16).unwrap();

            sna.log(format!(
                "Token {token:?} set at value {value} (0x{value:x})"
            ));
        }
    }

    let fname = matches.get_one::<String>("OUTPUT").unwrap();
    let version = matches
        .get_one::<String>("version")
        .unwrap()
        .parse::<u8>()
        .unwrap()
        .try_into()
        .unwrap();
    sna.save(fname, version)
        .expect("Unable to save the snapshot");
}
