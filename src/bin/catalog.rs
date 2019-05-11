/// Catalog tool manipulator.
/// 
extern crate clap;
extern crate cpclib;
extern crate log;
extern crate simplelog;

use clap::{App, Arg, ArgGroup, SubCommand};
use std::fs::File;
use std::io::{Read, Write};

use cpclib::disc::edsk::{ExtendedDsk, Side};
use cpclib::disc::amsdos::*;
use log::{info, trace, warn, error};
use simplelog::*;

fn main() -> std::io::Result<()> {
    TermLogger::init(LevelFilter::Debug, Config::default()).expect("Unable to build logger");

	let matches = App::new("catalog")
					.about("Amsdos catalog manipulation tool.")
					.author("Krusty/Benediction")
					.arg(
						Arg::with_name("LIST")
						.help("List the content of the catalog ONLY for files having no control chars")
						.long("list")
						.short("l")
					)
					.arg(
						Arg::with_name("LISTALL")
						.help("List the content of the catalog EVEN for files having no control chars")
						.long("listall")
						.short("a")
					)
					.arg(
						Arg::with_name("INPUT_FILE")
						.help("Input/Output file that contains the entries of the catalog (a binary file or a dsk)")
						.takes_value(true)
						.required(true)
						.long("input")
						.short("i")
					)
					.arg(
						Arg::with_name("ENTRY")
						.help("Selects the entry to modify")
						.takes_value(true)
						.long("entry")
						.validator(|v|{
							v.parse::<u8>()
							.map_err(|e|{e.to_string()})
							.and_then(
								|nb|
								if nb >= 0 && nb <= 63 {
									Ok(())
								}
								else {
									Err("The entry must be a number between 0 and 63".to_owned())
								}
							)
						})
					)
					.arg(
						Arg::with_name("SETREADONLY")
							.help("Set the selected entry readonly")
							.long("readonly")
							.requires("ENTRY")
					)
					.arg(
						Arg::with_name("SETSYSTEM")
							.help("Set the selected entry hidden")
							.long("system")
							.requires("ENTRY")
					)
					.arg(
						Arg::with_name("UNSETREADONLY")
							.help("Set the selected entry read and write")
							.long("noreadonly")
							.requires("ENTRY")
					)
					.arg(
						Arg::with_name("UNSETSYSTEM")
							.help("Set the selected entry visible")
							.long("nosystem")
							.requires("ENTRY")
					)
					.get_matches();


	// Retrieve the current entries ...
	let catalog_fname = matches.value_of("INPUT_FILE").unwrap();
	let mut catalog_content:AmsdosEntries = {
		let mut content = Vec::new();

		if catalog_fname.contains("dsk") {
			// Read a dsk file
			error!("Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results.");
			let dsk = ExtendedDsk::open(catalog_fname).expect("unable to read the dsk file");
			let manager = AmsdosManager::new_from_disc(dsk, Side::SideA);
			manager.catalog()
		}
		else {
			// Read a catalog file
			let mut file = File::open(catalog_fname)?;
			file.read_to_end(&mut content)?;
			AmsdosEntries::from_slice(&content)
		}
	};

	// ... and manipulate them
	if matches.is_present("LIST") || matches.is_present("LISTALL") {
		let listall = matches.is_present("LISTALL");
		for (idx, entry) in catalog_content.all_entries().enumerate() {
			let is_present = !entry.is_erased();
			let is_hidden = entry.is_system();
			let is_read_only = entry.is_read_only();

			let fname = entry.format();
			let contains_control_chars = !fname.as_str().chars().map(|c|{
				c.is_ascii_graphic()
			}).all(|t| t == true);

			if is_present && !contains_control_chars {
				print!("{}. {}", idx, fname);
				if is_hidden {
					print!(" [hidden]");
				}
				if is_read_only {
					print!(" [read only]");
				}
				println!("");
			} else if is_present && contains_control_chars && listall {
				println!("{}. => CONTROL CHARS <=", idx);
			}
			else if !is_present {
				println!("{}. => EMPTY SLOT <=", idx);
			}
		}

	}

	if let Some(idx) = matches.value_of("ENTRY") {
		let idx = idx.parse::<u8>().unwrap();
		info!("Manipulate entry {}", idx);

		let mut entry = catalog_content.get_entry_mut(idx as _);
		if matches.is_present("SETREADONLY") {
			entry.set_read_only();
		}
		if matches.is_present("SETSYSTEM") {
			entry.set_system();
		}
		if matches.is_present("UNSETREADONLY") {
			entry.unset_read_only();
		}
		if matches.is_present("UNSETSYSTEM") {
			entry.unset_system();
		}

		// Write the result 
		if catalog_fname.contains("dsk") {
			unimplemented!("Need to implement that");
		}
		else {
			let mut file = File::create(catalog_fname)?;
			file.write_all(&catalog_content.as_bytes())?;
		}
	}

	Ok(())
}