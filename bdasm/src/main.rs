use clap;
use clap::{App, Arg};
use std::fs::File;
use std::io::{Read};

use cpclib_asm::preamble::*;
use cpclib_disc::amsdos::AmsdosHeader;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );
    let matches = App::new("bdasm")
					.version(built_info::PKG_VERSION)
					.author("Krusty/Benediction")
					.about("Benediction disassembler")
					.before_help(&desc_before[..])
					.arg(
						Arg::with_name("INPUT")
							.help("Input binary file to disassemble.")
							.takes_value(true)
							.required(true)
					)
					.arg(
						Arg::with_name("ORIGIN")
							.help("Disassembling origin (ATTENTION hexadecimal only)")
							.short("o")
							.long("origin")
							.takes_value(true)
							.required(false)

					)
					.arg(
						Arg::with_name("DATA_BLOC")
						.help("Relative position that contains data for a given size. Format: RELATIVE_START(in hexadecimal)-SIZE(in decimal)")
						.short("d")
						.long("data")
						.takes_value(true)
						.number_of_values(1)
						.multiple(true)
					)
					.arg(
						Arg::with_name("LABEL")
						.help("Set a label at the given address. Format LABEL:ADDRESS(in hexadecimal")
						.short("l")
						.long("label")
						.takes_value(true)
						.number_of_values(1)
						.multiple(true)
					)
                    .arg(
                        Arg::with_name("SKIP")
                        .help("Skip the first <SKIP> bytes")
                        .short("s")
                        .long("SKIP")
                        .takes_value(true)
                        .number_of_values(1)
                        .multiple(false)
                    )
                    .arg(
                        Arg::with_name("COMPRESS")
                        .help("Output a simple listing that only contains the opcodes")
                        .short("c")
                        .long("compressed")
                    )
					.get_matches();

    // Get the bytes to disassemble
    let input_filename = matches.value_of("INPUT").unwrap();
    let mut input_bytes = Vec::new();
    let mut file = File::open(input_filename).expect("Unable to open file");
    file.read_to_end(&mut input_bytes)
        .expect("Unable to read file");

    // check if there is an amsdos header and remove it if any
    let (input_bytes, amsdos_load) = if input_bytes.len() > 128 {
        let header = AmsdosHeader::from_buffer(&input_bytes);
        if header.is_checksum_valid() {
            println!("Amsdos header detected and removed");
            (&input_bytes[128..], Some(header.loading_address()))
        } else {
            (input_bytes.as_ref(), None)
        }
    } else {
        (input_bytes.as_ref(), None)
    };

    // check if first bytes need to be removed
    let input_bytes = if let Some(skip) = matches.value_of("SKIP") {
        let skip = skip.parse::<usize>().expect("Unable to convert SKIP value");
        eprintln!("; Skip {} bytes", skip);
        &input_bytes[skip..]
    } else {
        input_bytes
    };

    // Disassemble
    eprintln!("; 0x{:x} bytes to disassemble", input_bytes.len());

    // Retreive the listing
    // TODO move that in its own function
    let mut listing: Listing = if matches.is_present("DATA_BLOC") {
        // retreive the blocs and parse them
        let mut blocs = matches
            .values_of("DATA_BLOC")
            .unwrap()
            .map(|bloc| {
                let split = bloc.split("-").collect::<Vec<_>>();
                let start = usize::from_str_radix(split[0], 16).unwrap();
                let length = match usize::from_str_radix(split[1], 10) {
                    Ok(l) => Some(l),
                    Err(_) => None,
                };
                (start, length)
            })
            .collect::<Vec<_>>();
        blocs.sort();

        // make the listing for each of the blocs
        let mut listings: Vec<Listing> = Vec::new();
        let mut current_idx = 0;
        while !blocs.is_empty() {
            let &(bloc_idx, bloc_length) = blocs.first().unwrap();
            if current_idx < bloc_idx {
                listings.push(cpclib_asm::disass::disassemble(
                    &input_bytes[current_idx..(bloc_idx - current_idx)],
                ));
                current_idx = bloc_idx;
            } else {
                assert_eq!(current_idx, bloc_idx);
                listings.push(
                    defb_elements(match bloc_length {
                        Some(l) => &input_bytes[current_idx..(current_idx + l)],
                        None => &input_bytes[current_idx..],
                    })
                    .into(),
                );
                blocs.remove(0);
                current_idx += match bloc_length {
                    Some(l) => l,
                    None => input_bytes.len() - current_idx,
                };
            }
        }
        if current_idx < input_bytes.len() {
            listings.push(cpclib_asm::disass::disassemble(&input_bytes[current_idx..]));
        }

        // merge the blocs
        listings
            .into_iter()
            .fold(Listing::new(), |mut lst, current| {
                lst.inject_listing(&current);
                lst
            })
    } else {
        // no blocs
        cpclib_asm::disass::disassemble(&input_bytes)
    };

    // add origin if any
    if let Some(address) = matches.value_of("ORIGIN") {
        let origin = u16::from_str_radix(address, 16).unwrap();
        listing.insert(0, org(origin));
    } else {
        if let Some(origin) = amsdos_load {
            listing.insert(0, org(origin));
        }
    }

    // add labels
    if let Some(labels) = matches.values_of("LABEL") {
        let mut labels = labels
            .map(|label| {
                let split = label.split(':').collect::<Vec<_>>();
                assert_eq!(2, split.len());
                let label = split[0];
                let address = u16::from_str_radix(split[1], 16).unwrap();
                (address, label)
            })
            .collect::<Vec<_>>();
        labels.sort();

        listing.inject_labels(&labels);
    }

    if matches.is_present("COMPRESS") {
        println!("{}", listing.to_string());
    } else {
        println!("{}", listing.to_enhanced_string());
    }
}
