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
#![cfg(feature = "catalog")]

use std::fs::File;
use std::io::{Read, Write};

/// Catalog tool manipulator.
use cpclib_common::clap::{Arg, ArgAction, Command, value_parser};
use cpclib_common::num::Num;
use cpclib_disc::amsdos::{AmsdosEntries, AmsdosManagerNonMut, BlocIdx};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::{ExtendedDsk, Head};
use log::{error, info};
use simple_logger::{SimpleLogger, set_up_color_terminal};
#[must_use]
///
/// # Panics
///
/// Panics if the string cannot be parsed as a number in the expected format.
pub fn to_number<T>(repr: &str) -> T
where
    T: Num,
    T::FromStrRadixErr: std::fmt::Debug
{
    dbg!(repr);
    let repr = repr.trim();
    if let Some(stripped) = repr.strip_prefix("0x") {
        T::from_str_radix(stripped, 16)
    }
    else if let Some(stripped) = repr.strip_prefix("\\$") {
        T::from_str_radix(stripped, 16)
    }
    else if let Some(stripped) = repr.strip_prefix('&') {
        T::from_str_radix(stripped, 16)
    }
    else if repr.starts_with('0') {
        T::from_str_radix(repr, 8)
    }
    else {
        T::from_str_radix(repr, 10)
    }
    .expect("Unable to parse number")
}

#[allow(clippy::too_many_lines)]
fn main() -> std::io::Result<()> {
    // XXX this has been disabled for compatbility reasons with gpu
    // XXX as this software has been used since ages, I have no idea if this is an issue or not
    // TermLogger::init(
    // LevelFilter::Debug,
    // Config::default(),
    // TerminalMode::Mixed,
    // ColorChoice::Auto
    // )
    // .expect("Unable to build logger");
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(logger)).unwrap();

    let matches = Command::new("catalog")
					.about("Amsdos catalog manipulation tool.")
					.author("Krusty/Benediction")
					.arg(
						Arg::new("LIST")
						.help("List the content of the catalog ONLY for files having no control chars")
						.long("list")
						.short('l')
                        .action(ArgAction::SetTrue)
					)
					.arg(
						Arg::new("LISTALL")
						.help("List the content of the catalog EVEN for files having no control chars")
						.long("listall")
						.short('a')
                        .action(ArgAction::SetTrue)
					)
					.arg(
						Arg::new("INPUT_FILE")
						.help("Input/Output file that contains the entries of the catalog (a binary file or a dsk)")
						.required(true)
						.long("input")
						.short('i')
					)
					.arg(
						Arg::new("ENTRY")
						.help("Selects the entry to modify")
						.long("entry")
                        .value_parser(value_parser!(u8).range(..=63))
					)
					.arg(
						Arg::new("SETREADONLY")
							.help("Set the selected entry readonly")
							.long("readonly")
							.requires("ENTRY")
                        .action(ArgAction::SetTrue)
					)
					.arg(
						Arg::new("SETSYSTEM")
							.help("Set the selected entry hidden")
							.long("system")
							.requires("ENTRY")
                        .action(ArgAction::SetTrue)

					)
					.arg(
						Arg::new("UNSETREADONLY")
							.help("Set the selected entry read and write")
							.long("noreadonly")
							.requires("ENTRY")
                        .action(ArgAction::SetTrue)

					)
					.arg(
						Arg::new("UNSETSYSTEM")
							.help("Set the selected entry visible")
							.long("nosystem")
							.requires("ENTRY")
                        .action(ArgAction::SetTrue)

					)
					.arg(
						Arg::new("USER")
							.help("Set the user value")
							.long("user")
							.requires("ENTRY")
							.value_parser(value_parser!(u8))
					)
					.arg(
						Arg::new("FILENAME")
							.help("Set the filename of the entry")
							.long("filename")
							.requires("ENTRY")
					)
					.arg(
						Arg::new("BLOCS")
							.help("Set the blocs to load (and update the number of blocs accordingly to that)")
							.long("blocs")
							.requires("ENTRY")
							.value_parser(value_parser!(u8))
							.num_args(..=16)
					)
					.arg(
						Arg::new("NUMPAGE")
						.help("Set the page number")
						.long("numpage")
					)
					.arg(
						Arg::new("SIZE")
						.help("Force the size of the entry")
						.long("size")
					)
					.get_matches();

    // Retrieve the current entries ...
    let catalog_fname = matches.get_one::<String>("INPUT_FILE").unwrap();
    let mut catalog_content: AmsdosEntries = {
        let mut content = Vec::new();

        if catalog_fname.contains("dsk") {
            // Read a dsk file
            error!(
                "Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results."
            );
            let dsk = ExtendedDsk::open(catalog_fname).expect("unable to read the dsk file");
            let manager = AmsdosManagerNonMut::new_from_disc(&dsk, Head::A);
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
    if matches.contains_id("LIST") || matches.contains_id("LISTALL") {
        let listall = matches.contains_id("LISTALL");
        for (idx, entry) in catalog_content.all_entries().enumerate() {
            let contains_id = !entry.is_erased();
            let is_hidden = entry.is_system();
            let is_read_only = entry.is_read_only();

            let fname = entry.format();
            let contains_control_chars = !fname.as_str().chars().all(|c| c.is_ascii_graphic());

            if contains_id && !contains_control_chars {
                print!("{idx}. {fname}");
                if is_hidden {
                    print!(" [hidden]");
                }
                if is_read_only {
                    print!(" [read only]");
                }

                print!(" {}Kb {:?}", entry.used_space(), entry.used_blocs());
                println!();
            }
            else if contains_id && contains_control_chars && listall {
                println!("{idx}. => CONTROL CHARS <=");
            }
            else if !contains_id {
                println!("{idx}. => EMPTY SLOT <=");
            }
        }
    }

    if let Some(idx) = matches.get_one::<String>("ENTRY") {
        let idx = idx.parse::<u8>().unwrap();
        info!("Manipulate entry {idx}");

        let entry = catalog_content.get_entry_mut(idx as _);

        if matches.contains_id("SETREADONLY") {
            entry.set_read_only();
        }
        if matches.contains_id("SETSYSTEM") {
            entry.set_system();
        }
        if matches.contains_id("UNSETREADONLY") {
            entry.unset_read_only();
        }
        if matches.contains_id("UNSETSYSTEM") {
            entry.unset_system();
        }

        if let Some(user) = matches.get_one::<u8>("USER") {
            entry.set_user(*user);
        }

        if let Some(filename) = matches.get_one::<String>("FILENAME") {
            entry.set_filename(filename);
        }

        if let Some(blocs) = matches.get_many::<u8>("BLOCS") {
            let blocs = blocs
                .map(|bloc| BlocIdx::from(*bloc))
                .collect::<Vec<BlocIdx>>();
            entry.set_blocs(&blocs);
        }

        if let Some(numpage) = matches.get_one::<String>("NUMPAGE") {
            entry.set_num_page(to_number::<u8>(numpage));
        }

        // XXX It is important ot keep it AFTER the blocs as it override their value
        if let Some(size) = matches.get_one::<String>("SIZE") {
            let size = to_number::<u8>(size);
            entry.set_page_size(size);
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
