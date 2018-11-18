extern crate cpc;
extern crate bitsets;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::str::FromStr;
use std::path::Path;
use std::io::BufReader;
use std::fmt;

use cpc::sna::*;

extern crate clap;
use clap::{Arg, App, SubCommand};

pub mod built_info {
include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

extern crate built;
extern crate time;
extern crate semver;


/**
 * Convert a string to its unsigned 32 bits representation (to access to extra memory)
 */
fn string_to_nb(source: &str) -> u32 {
    let error =format!("Unable to read the value: {}", source);
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    }
    else {
        source.parse::<u32>().expect(&error)
    }
}



fn main() {
    eprintln!("[WARNING] This is still a draft version that implement still few functionnalities");

    let desc_before = format!("Profile {} compiled: {}", built_info::PROFILE, built_info::BUILT_TIME_UTC);
    let matches = App::new("createSnapshot")
                          .version(built_info::PKG_VERSION)
                          .author("Krusty/Benediction")
                          .about("Amstrad CPC snapshot manipulation")
                          .before_help(&desc_before[..])
                          .after_help("This tool tries to be similar than Ramlaid's one")
                          .arg(Arg::with_name("info")
                               .help("Display informations on the loaded snapshot")
                               .multiple(false)
                               .long("info")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::with_name("debug")
                            .help("Display debugging information while manipulating the snapshot")
                            .long("debug")
                          )
                          .arg(Arg::with_name("OUTPUT")
                               .help("Sets the output file to generate")
                               .conflicts_with("flags")
                               .conflicts_with("info")
                               .conflicts_with("getToken")
                               .last(true)
                               .required(true))
                          .arg(Arg::with_name("inSnapshot")
                               .takes_value(true)
                               .short("i")
                               .long("inSnapshot")
                               .value_name("INFILE")
                               .multiple(false)
                               .number_of_values(1)
                               .help("Load <INFILE> snapshot file")
                               )
                          .arg(Arg::with_name("load")
                               .takes_value(true)
                               .short("l")
                               .long("load")
                               .multiple(true)
                               .number_of_values(2)
                               .help("Specify a file to include. -l fname address"))
                          .arg(Arg::with_name("getToken")
                               .takes_value(true)
                               .short("g")
                               .long("getToken")
                               .multiple(true)
                               .number_of_values(1)
                               .help("Display the value of a snapshot token")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::with_name("setToken")
                               .takes_value(true)
                               .short("s")
                               .long("setToken")
                               .multiple(true)
                               .number_of_values(2)
                               .help("Set snapshot token <$1> to value <$2>\nUse <$1>:<val> to set array value\n\t\tex '-s CRTC_REG:6 20' : Set CRTC register 6 to 20"))
                          .arg(Arg::with_name("putData")
                               .takes_value(true)
                               .short("p")
                               .long("putData")
                               .multiple(true)
                               .number_of_values(2)
                               .help("Put <$2> byte at <$1> address in snapshot memory")

                            )
                          .arg(Arg::with_name("flags")
                                .help("List the flags and exit")
                               .long("flags"))
                          .get_matches();




    // Display all tokens
    if matches.is_present("flags") {
        for flag in SnapshotFlag::enumerate().into_iter() {
            println!("{:?} / {:?} bytes.{}", flag, flag.elem_size(), flag.comment());
        }
        return;
    }


    let mut sna = if matches.is_present("inSnapshot"){
        let fname = matches.value_of("inSnapshot").unwrap();
        let path = Path::new(&fname);
        Snapshot::load(path)
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
        for i in 0..(loads.len()/2) {
            let fname = loads[i*2+0];
            let place = loads[i*2+1];

            let address = {
                if place.starts_with("0x") {
                    u32::from_str_radix(&place[2..], 16).unwrap()
                }
                else if place.starts_with("0") {
                    u32::from_str_radix(&place[1..],8).unwrap()
                }
                else {
                    place.parse::<u32>().unwrap()
                }
            };
            sna.add_file(fname, address as usize).expect("Unable to add file");
        }
    }



    // Patch memory
    if matches.is_present("putData") {
        let data = matches.values_of("putData").unwrap().collect::<Vec<_>>();

        for i in 0..(data.len()/2) {
            let address = string_to_nb(data[i*2+0]);
            let value = string_to_nb(data[i*2+1]);
            assert!(value < 0x100);

            sna.set_memory(address, value as u8);
        }
    }

    // Read the tokens
    if matches.is_present("getToken") {
        for token in matches.values_of("getToken").unwrap() {
            let mut token = SnapshotFlag::from_str(token).unwrap();
            println!("{:?} => {}", &token, sna.get_value(&token));
        }
        return;
    }

    // Set the tokens
    if matches.is_present("setToken") {
        let loads = matches.values_of("setToken").unwrap().collect::<Vec<_>>();
        for i in 0..(loads.len()/2) {

            // Read the parameters from the command line
            let token = loads[i*2+0];
            let (token, _index) = if token.contains(":") {
                let elems = token.split(":").collect::<Vec<_>>();
                (elems[0], Some(elems[1].parse::<usize>().expect("Unable to read indice")))
            }
            else {
                (token, None)
            };
            let value = {
                let source =loads[i*2+1];
                string_to_nb(source)
            };

            // Get the token
            let token = SnapshotFlag::from_str(token).unwrap();
            sna.set_value(token, value as u16);


            sna.log(format!("Token {:?} set at value {} (0x{:x})", token, value, value));
        }
    }


    let fname = matches.value_of("OUTPUT").unwrap();
    sna.save_sna(&fname);

}
