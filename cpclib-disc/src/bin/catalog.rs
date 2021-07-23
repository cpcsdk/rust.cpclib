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

/// Catalog tool manipulator.
///
use clap::{App, Arg};
use std::fs::File;
use std::io::{Read, Write};

use cpclib_disc::amsdos::*;
use cpclib_disc::edsk::{ExtendedDsk, Head};
use log::{error, info};
use num::Num;
use simplelog::*;

pub fn to_number<T>(repr: &str) -> T
where
    T: Num,
    <T as num::Num>::FromStrRadixErr: std::fmt::Debug,
{
    dbg!(repr);
    let repr = repr.trim();
    let repr = &repr;
    if repr.starts_with("0x") {
        T::from_str_radix(dbg!(&repr[2..]), 16)
    } else if repr.starts_with("\\$") || repr.starts_with('&') {
        T::from_str_radix(dbg!(&repr[1..]), 16)
    } else if repr.starts_with('0') {
        T::from_str_radix(dbg!(repr), 8)
    } else {
        T::from_str_radix(dbg!(repr), 10)
    }
    .expect("Unable to parse number")
}

fn main() -> std::io::Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Unable to build logger");

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
								if nb <= 63 {
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
					.arg(
						Arg::with_name("USER")
							.help("Set the user value")
							.long("user")
							.takes_value(true)
							.requires("ENTRY")
							.validator(|v|{
								v.parse::<u8>() // between 0 and 255
								.map_err(|e|{e.to_string()})
								.and_then(
									|_nb|
									Ok(())
								)
							})
					)
					.arg(
						Arg::with_name("FILENAME")
							.help("Set the filename of the entry")
							.takes_value(true)
							.long("filename")
							.requires("ENTRY")
					)
					.arg(
						Arg::with_name("BLOCS")
							.help("Set the blocs to load (and update the number of blocs accordingly to that)")
							.long("blocs")
							.takes_value(true)
							.requires("ENTRY")
							.multiple(false)
							.max_values(16)
					)
					.arg(
						Arg::with_name("NUMPAGE")
						.help("Set the page number")
						.long("numpage")
						.takes_value(true)
					)
					.arg(
						Arg::with_name("SIZE")
						.help("Force the size of the entry")
						.long("size")
						.takes_value(true)
					)
					.get_matches();

    // Retrieve the current entries ...
    let catalog_fname = matches.value_of("INPUT_FILE").unwrap();
    let mut catalog_content: AmsdosEntries = {
        let mut content = Vec::new();

        if catalog_fname.contains("dsk") {
            // Read a dsk file
            error!("Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results.");
            let dsk = ExtendedDsk::open(catalog_fname).expect("unable to read the dsk file");
            let manager = AmsdosManager::new_from_disc(dsk, Head::A);
            manager.catalog()
        } else {
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
            let contains_control_chars = !fname
                .as_str()
                .chars()
                .map(|c| c.is_ascii_graphic())
                .all(|t| t);

            if is_present && !contains_control_chars {
                print!("{}. {}", idx, fname);
                if is_hidden {
                    print!(" [hidden]");
                }
                if is_read_only {
                    print!(" [read only]");
                }

                print!(" {}Kb {:?}", entry.used_space(), entry.used_blocs());
                println!();
            } else if is_present && contains_control_chars && listall {
                println!("{}. => CONTROL CHARS <=", idx);
            } else if !is_present {
                println!("{}. => EMPTY SLOT <=", idx);
            }
        }
    }

    if let Some(idx) = matches.value_of("ENTRY") {
        let idx = idx.parse::<u8>().unwrap();
        info!("Manipulate entry {}", idx);

        let entry = catalog_content.get_entry_mut(idx as _);

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

        if let Some(user) = matches.value_of("USER") {
            let user = to_number::<u8>(user);
            entry.set_user(user);
        }

        if let Some(filename) = matches.value_of("FILENAME") {
            entry.set_filename(filename);
        }

        if let Some(blocs) = matches.values_of("BLOCS") {
            let blocs = blocs
                .map(|bloc| BlocIdx::from(to_number::<u8>(bloc)))
                .collect::<Vec<BlocIdx>>();
            entry.set_blocs(&blocs);
        }

        if let Some(numpage) = matches.value_of("NUMPAGE") {
            entry.set_num_page(to_number::<u8>(numpage));
        }

        // XXX It is important ot keep it AFTER the blocs as it override their value
        if let Some(size) = matches.value_of("SIZE") {
            let size = to_number::<u8>(size);
            entry.set_page_size(size);
        }

        // Write the result
        if catalog_fname.contains("dsk") {
            unimplemented!("Need to implement that");
        } else {
            let mut file = File::create(catalog_fname)?;
            file.write_all(&catalog_content.as_bytes())?;
        }
    }

    Ok(())
}
