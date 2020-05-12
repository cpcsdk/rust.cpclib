
use clap;
use clap::{App, Arg, ArgGroup, ArgMatches};
use std::fs::File;
use std::io::{Read, Write};

use cpclib_asm::preamble::*;

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
					.get_matches();
	
	// Get the bytes to disassemble
	let input_filename = matches.value_of("INPUT").unwrap();
	let mut input_bytes = Vec::new();
	let mut file = File::open(input_filename).expect("Unable to open file");
	file.read_to_end(&mut input_bytes).expect("Unable to read file");

	// Disassemble
	eprintln!("0x{:x} bytes to disassemble", input_bytes.len());
	let listing = {
		let mut listing = cpclib_asm::disass::disassemble(&input_bytes);
		if let Some(address) = matches.value_of("ORIGIN") {
			let origin = u16::from_str_radix(address, 16).unwrap();
			listing.insert(0, org(origin));
		};
		listing
	};

	println!("{}", listing.to_enhanced_string());

}