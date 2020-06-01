
use clap;
use clap::{App, Arg, ArgGroup, ArgMatches};
use std::fs::File;
use std::io::{Read, Write};

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
						.help("Relative position that contains data RELATIVE_START-SIZE")
						.short("d")
						.long("data")
						.takes_value(true)
						.number_of_values(1)
						.multiple(true)
					)
					.get_matches();
	
	// Get the bytes to disassemble
	let input_filename = matches.value_of("INPUT").unwrap();
	let mut input_bytes = Vec::new();
	let mut file = File::open(input_filename).expect("Unable to open file");
	file.read_to_end(&mut input_bytes).expect("Unable to read file");

	// check if there is an amsdos header and remove it if any
	let (input_bytes, amsdos_load) = if input_bytes.len() > 128 {
		let header = AmsdosHeader::from_buffer(&input_bytes) ;
		if header.is_checksum_valid() {
			println!("Amsdos header detected and removed");
			(&input_bytes[128..], Some(header.loading_address()))
		}
		else {
			(input_bytes.as_ref(), None)
		}
	}
	else {
		(input_bytes.as_ref(), None)
	};
	
	// Disassemble
	eprintln!("0x{:x} bytes to disassemble", input_bytes.len());

	// Retreive the listing
	// TODO move that in its own function
	let mut listing : Listing = if matches.is_present("DATA_BLOC") {
		// retreive the blocs and parse them
		let mut blocs = matches
							.values_of("DATA_BLOC")
							.unwrap()
							.map(|bloc|{
								let split = bloc.split("-").collect::<Vec<_>>();
								(
									usize::from_str_radix(split[0], 16).unwrap(), // start in hexa
									usize::from_str_radix(split[1], 10).unwrap(), // lenght in decimal
								)
							})
							.collect::<Vec<_>>();
		blocs.sort();

		// make the listing for each of the blocs
		let mut listings: Vec<Listing> = Vec::new();
		let mut current_idx = 0;
		while ! blocs.is_empty() {
			let &(bloc_idx, bloc_length) = blocs.first().unwrap();
			if current_idx < bloc_idx {
				listings.push(
					cpclib_asm::disass::disassemble(
						&input_bytes[current_idx..(bloc_idx-current_idx)]
					)
				);
				current_idx = bloc_idx;
			}
			else {
				assert_eq!(current_idx, bloc_idx);
				listings.push(
					defb_elements(
						&input_bytes[current_idx..(current_idx+bloc_length)]).into()
				);
				blocs.remove(0);
				current_idx += bloc_length;
			}
		}
		if current_idx < input_bytes.len() {
			listings.push(
				cpclib_asm::disass::disassemble(
					&input_bytes[current_idx..]
				)
			);
		}

		// merge the blocs
		listings.into_iter().fold(
			Listing::new(),
			|mut lst, current| {
				lst.inject_listing(&current);
				lst
			}
		)
	}
	else {
		// no blocs
		cpclib_asm::disass::disassemble(&input_bytes)
	};

	// add origin if any
	if let Some(address) = matches.value_of("ORIGIN") {
		let origin = u16::from_str_radix(address, 16).unwrap();
		listing.insert(0, org(origin));
	}
	else {
		if let Some(origin) = amsdos_load {
			listing.insert(0, org(origin));
		}
	}
	

	println!("{}", listing.to_enhanced_string());

}