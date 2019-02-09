
extern crate bitsets;

use std::io;
use std::fs::File;
use std::io::{Read,Write};
use std::str::FromStr;
use std::path::Path;
use std::io::BufReader;
use std::fmt;

use cpclib::assembler::*;
use cpclib::disc::amsdos::{AmsdosFileName, AmsdosManager};
use cpclib::assembler::tokens::Listing;
use cpclib::assembler::parser::*;
use cpclib::assembler::assembler::Env;
use cpclib::assembler::AssemblerError;

extern crate clap;
use clap::{App, ArgMatches, ArgGroup, Arg};

pub mod built_info {
include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

extern crate built;
extern crate time;
extern crate semver;
#[macro_use] extern crate failure;

use failure::Error;

#[derive(Debug, Fail)]
enum BasmError {
    #[fail(display = "IO error: {}", io)]
	Io {
		io: io::Error
	},
    #[fail(display = "Assembling error: {}", error)]
	AssemblerError {
		error: AssemblerError
	},
    #[fail(display = "Invalid Amsdos filename: {}", filename)]
	InvalidAmsdosFilename {
		filename: String
	}


}

// XXX I do not understand why I have to do that !!!
impl From<std::io::Error> for BasmError {
	fn from(error: std::io::Error) -> BasmError {
		BasmError::Io{io: error}
	}
}

impl From<AssemblerError> for BasmError {
	fn from(error: AssemblerError) -> BasmError {
		BasmError::AssemblerError{error: error}
	}
}



/// Parse the given code.
/// TODO read options to configure the search path
fn parse (matches: &ArgMatches) -> Result<Listing, BasmError> {
	let filename = matches.value_of("INPUT").unwrap();

	let code = {
		let mut f = File::open(filename)?;
		let mut content = String::new();
		f.read_to_string(&mut content);
		content
	};

	let mut context = ParserContext::default();
	context.set_current_filename(&filename);
	context.add_search_path_from_file(&filename);

	parse_str_with_context(&code, &context)
		.map_err(|e|{e.into()})
}

/// Assemble the given code
/// TODO use options to configure the base symbole table
fn assemble(matches: &ArgMatches, listing: &Listing) -> Result<Env, BasmError> {
	let mut options = AssemblingOptions::default();

	options.set_case_sensitive(!matches.is_present("CASE_INSENSITIVE"));

	// TODO add symbols if any

	crate::assembler::visit_tokens_all_passes_with_options(
		&listing.listing(),
		&options
	).map_err(|e|{e.into()})
}

/// Save the provided result
/// TODO manage the various save options
fn save(matches: &ArgMatches, env: &Env) -> Result<(), BasmError> {
	let single_binary = true;
	if single_binary {
		let pc_filename = matches.value_of("OUTPUT").unwrap();
		let amsdos_filename = AmsdosFileName::from(pc_filename);

		// Raise an error if the filename is not compatible with the header
		if matches.is_present("HEADER") && !amsdos_filename.is_valid() {
			return Err(
				BasmError::InvalidAmsdosFilename{filename: pc_filename.to_string()}
			)
		}

		// Collect the produced bytes
		let binary = env.produced_bytes();

		// Compute the headers if needed
		let header = if matches.is_present("BINARY_HEADER") {
			AmsdosManager::compute_binary_header(
				&amsdos_filename,
				env.loading_address().unwrap() as  u16,
				env.execution_address().unwrap() as u16,
				&binary
			).as_bytes().to_vec()
		}
		else if matches.is_present("BASIC_HEADER") {
			AmsdosManager::compute_basic_header(
				&amsdos_filename,
				&binary
			).as_bytes().to_vec()
		}
		else {
			Vec::new()
		};

		// Save file on disc
		let mut f = File::create(pc_filename)?;
		if header.len() > 0 {
			f.write_all(&header)?;
		}
		f.write_all(&binary)?;
	}

	Ok(())

}

fn process(matches: &ArgMatches) -> Result<(), BasmError>{

	let listing = parse(matches)?;
	let env = assemble(matches, &listing)?;
	save(matches, &env)
}

fn main() {

    let desc_before = format!("Profile {} compiled: {}", built_info::PROFILE, built_info::BUILT_TIME_UTC);
    let matches = App::new("basm")
					.version(built_info::PKG_VERSION)
					.author("Krusty/Benediction")
					.about("Benediction ASM -- z80 assembler that taylor Amastrad CPC")
					.before_help(&desc_before[..])
					.after_help("Work In Progress")
					.arg(
						Arg::with_name("OUTPUT")
							.help("Filename of the output.")
							.short("o")
							.long("output")
							.takes_value(true)
							.required(true)
					)
					.arg(
						Arg::with_name("INPUT")
							.help("Input file to read.")
							.takes_value(true)
							.required(true)
					)
					.arg(
						Arg::with_name("BASIC_HEADER")
							.help("Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive.")
							.long("basic")
							.alias("basicheader")
					)
					.arg(
						Arg::with_name("BINARY_HEADER")
							.help("Request a binary header")
							.long("binary")
							.alias("header")
							.alias("binaryheader")
					)
					.arg(
						Arg::with_name("CASE_INSENSITIVE")
							.help("Configure the assembler to be case insensitive")
							.long("case-insensitive")
							.short("i") 
					)
					.group(
						ArgGroup::with_name("HEADER")
							.args(&["BINARY_HEADER", "BASIC_HEADER"])
					)
					.get_matches();
	process(&matches).expect("Error while assembling file.");
}

