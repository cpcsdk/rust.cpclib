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

use cpclib_common::clap::{Arg, Command};
use cpclib_sna::*;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Convert a string to its unsigned 32 bits representation (to access to extra memory)
pub fn string_to_nb(source: &str) -> u32 {
    let error = format!("Unable to read the value: {}", source);
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    }
    else if source.starts_with("%") {
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
                          .author("Krusty/Benediction")
                          .about("Amstrad CPC snapshot manipulation")
                          .before_help(&desc_before[..])
                          .after_help("This tool tries to be similar than Ramlaid's one")
                          .arg(Arg::new("info")
                               .help("Display informations on the loaded snapshot")
                               .long("info")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::new("debug")
                            .help("Display debugging information while manipulating the snapshot")
                            .long("debug")
                          )
                          .arg(Arg::new("OUTPUT")
                               .help("Sets the output file to generate")
                               .conflicts_with("flags")
                               .conflicts_with("info")
                               .conflicts_with("getToken")
                               .last(true)
                               .required(true))
                          .arg(Arg::new("inSnapshot")
                               .takes_value(true)
                               .short('i')
                               .long("inSnapshot")
                               .value_name("INFILE")
                               .number_of_values(1)
                               .help("Load <INFILE> snapshot file")
                               )
                          .arg(Arg::new("load")
                               .takes_value(true)
                               .short('l')
                               .long("load")
                               .multiple_occurrences(true)
                               .number_of_values(2)
                               .help("Specify a file to include. -l fname address"))
                          .arg(Arg::new("getToken")
                               .takes_value(true)
                               .short('g')
                               .long("getToken")
                               .multiple_occurrences(true)
                               .number_of_values(1)
                               .help("Display the value of a snapshot token")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::new("setToken")
                               .takes_value(true)
                               .short('s')
                               .long("setToken")
                               .multiple_occurrences(true)
                               .number_of_values(2)
                               .help("Set snapshot token <$1> to value <$2>\nUse <$1>:<val> to set array value\n\t\tex '-s CRTC_REG:6 20' : Set CRTC register 6 to 20"))
                          .arg(Arg::new("putData")
                               .takes_value(true)
                               .short('p')
                               .long("putData")
                               .multiple_occurrences(true)
                               .number_of_values(2)
                               .help("Put <$2> byte at <$1> address in snapshot memory")

                            )
                          .arg(Arg::new("version")
                                .takes_value(true)
                                .short('v')
                                .long("version")
                                .number_of_values(1)
                                .possible_values(&["1", "2", "3"])
                                .help("Version of the saved snapshot.")
                                .default_value("3")
                           )
                          .arg(Arg::new("flags")
                                .help("List the flags and exit")
                               .long("flags"))
                          .get_matches();

    // Display all tokens
    if matches.is_present("flags") {
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
    let mut sna = if matches.is_present("inSnapshot") {
        let fname = matches.value_of("inSnapshot").unwrap();
        let path = Path::new(&fname);
        Snapshot::load(path).expect("Error while loading the snapshot")
    }
    else {
        Snapshot::default()
    };

    // Activate the debug mode
    sna.debug = matches.is_present("debug");

    if matches.is_present("info") {
        sna.print_info();
        return;
    }

    // Manage the files insertion
    if matches.is_present("load") {
        let loads = matches.values_of("load").unwrap().collect::<Vec<_>>();
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
    if matches.is_present("putData") {
        let data = matches.values_of("putData").unwrap().collect::<Vec<_>>();

        for i in 0..(data.len() / 2) {
            let address = string_to_nb(data[i * 2 + 0]);
            let value = string_to_nb(data[i * 2 + 1]);
            assert!(value < 0x100);

            sna.set_byte(address, value as u8);
        }
    }

    // Read the tokens
    if matches.is_present("getToken") {
        for token in matches.values_of("getToken").unwrap() {
            let token = SnapshotFlag::from_str(token).unwrap();
            println!("{:?} => {}", &token, sna.get_value(&token));
        }
        return;
    }

    // Set the tokens
    if matches.is_present("setToken") {
        let loads = matches.values_of("setToken").unwrap().collect::<Vec<_>>();
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
                "Token {:?} set at value {} (0x{:x})",
                token, value, value
            ));
        }
    }

    let fname = matches.value_of("OUTPUT").unwrap();
    let version = matches
        .value_of("version")
        .unwrap()
        .parse::<u8>()
        .unwrap()
        .try_into()
        .unwrap();
    sna.save(&fname, version)
        .expect("Unable to save the snapshot");
}
