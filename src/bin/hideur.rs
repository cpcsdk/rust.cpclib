/// (unfinished) conversion of hideur maker


use std::fs::File;
use std::io::Read;
use std::io::Write;

use cpclib::disc::amsdos::*;
use cpclib::util::string_to_nb;

fn main() -> std::io::Result<()> {
	let matches = clap::App::new("hideur")
		.arg(
			clap::Arg::with_name("INPUT")
				.required(true)
				.help("Input file to manipulate")
		)
		.arg(
			clap::Arg::with_name("INFO")
				.long("info")
		)
		.arg(
			clap::Arg::with_name("OUTPUT")
				.short("o")
				.long("output")
				.required_unless("INFO")
				.help("Output file to generate")
				.takes_value(true)
		)
		.arg(
			clap::Arg::with_name("USER")
				.short("u")
				.long("user")
				.conflicts_with("INFO")
				.help("User where to put the file")
				.takes_value(true)
		)
		.arg(
			clap::Arg::with_name("TYPE")
				.short("t")
				.long("type")
				.conflicts_with("INFO")
				.required_unless("INFO")
				.help("File type")
				.case_insensitive(true)
				.possible_values(&[
					"0", "1", "2", 
					"Basic", "Protected", "Binary"])
				.takes_value(true)
		)
		.arg(
			clap::Arg::with_name("EXEC")
				.short("x")
				.long("execution")
				.conflicts_with("INFO")
				.help("Execution address")
				.takes_value(true)
		)
		.arg(
			clap::Arg::with_name("LOAD")
				.short("-l")
				.long("load")
				.conflicts_with("INFO")
				.help("Loading address")
				.takes_value(true)
		).get_matches();

	// Read the input file
	let content = {
		let input = matches.value_of("INPUT").unwrap();
		let mut f = File::open(input)?;
		let mut buf = Vec::new();
		f.read_to_end(&mut buf)?;
		buf
	};

	// Get the type of file
	let ftype = {
		match matches.value_of("TYPE").unwrap().to_ascii_lowercase().as_ref() {
			"0" | "basic" => AmsdosFileType::Basic,
			"1" | "protected" => AmsdosFileType::Protected,
			"2" | "binary" => AmsdosFileType::Binary,
			_ => unreachable!()
		}
	};

	// obtain its filename
	let filename = {
		let user = matches.value_of("USER").map_or(0, string_to_nb) as u8;
		let (filename, extension) = {
			let complete_filename = matches.value_of("INPUT").unwrap();
			let parts = complete_filename.split('.').collect::<Vec<_>>();
			match parts.len() {
				1 => (parts[0], ""),
				2 => (parts[0], parts[1]),
				_ => unreachable!()
			}
		};

		AmsdosFileName::new_correct_case(
			user,
			filename,
			extension
		).expect("Invalid file definition")
	};

	// Build the header according to the given options
	let header = match ftype {
		AmsdosFileType::Binary => {
			let exec = string_to_nb(& matches.value_of("EXEC").expect("The execution address is expected for a binary target")) as u16;
			let load = string_to_nb(& matches.value_of("LOAD").expect("The load address is expected for a binary target")) as u16;

			AmsdosManager::compute_binary_header(
				&filename, 
				load, 
				exec, 
				&content)
		},
		AmsdosFileType::Basic => {
			AmsdosManager::compute_basic_header(
				&filename,
				&content
			)
		},
		_ => unimplemented!()
	};

	// Write the final file
	let mut f = File::create(matches.value_of("OUTPUT").unwrap())?;
	f.write_all(header.as_bytes())?;
	f.write_all(&content)?;

	Ok(())
}