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
use clap::Parser;
use cpclib_common::clap::value_parser;
use cpclib_common::num::Num;
use cpclib_disc::amsdos::{AmsdosEntries, AmsdosManagerNonMut, BlocIdx};
use cpclib_disc::edsk::Head;
use log::{error, info};
use simple_logger::SimpleLogger;
use cpclib_disc::open_disc;
#[derive(Parser, Debug)]
#[command(name = "catalog")]
#[command(about = "Amsdos catalog manipulation tool.", author = "Krusty/Benediction")]
struct Args {
    /// List the content of the catalog ONLY for files having no control chars
    #[arg(short = 'l', long)]
    list: bool,

    /// List the content of the catalog EVEN for files having no control chars
    #[arg(short = 'a', long)]
    listall: bool,

    /// Input/Output file that contains the entries of the catalog (a binary file or a dsk)
    #[arg(short = 'i', long = "input")]
    input_file: String,

    /// Selects the entry to modify
    #[arg(long, value_parser = value_parser!(u8).range(..=63))]
    entry: Option<u8>,

    /// Set the selected entry readonly
    #[arg(long = "readonly", requires = "entry")]
    setreadonly: bool,

    /// Set the selected entry hidden
    #[arg(long = "system", requires = "entry")]
    setsystem: bool,

    /// Set the selected entry read and write
    #[arg(long = "noreadonly", requires = "entry")]
    unsetreadonly: bool,

    /// Set the selected entry visible
    #[arg(long = "nosystem", requires = "entry")]
    unsetsystem: bool,

    /// Set the user value
    #[arg(long, requires = "entry")]
    user: Option<u8>,

    /// Set the filename of the entry
    #[arg(long, requires = "entry")]
    filename: Option<String>,

    /// Set the blocs to load (and update the number of blocs accordingly to that)
    #[arg(long, requires = "entry", num_args = ..=16)]
    blocs: Option<Vec<u8>>,

    /// Set the page number
    #[arg(long)]
    numpage: Option<String>,

    /// Force the size of the entry
    #[arg(long)]
    size: Option<String>,
}
#[must_use]
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

fn list_catalog_entries(catalog_content: &AmsdosEntries, listall: bool) {
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

            print!(" {:>4}Kb {:?}", entry.used_space(), entry.used_blocs());
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
    let logger = SimpleLogger::new();
    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(logger)).unwrap();

    let args = Args::parse();

    // Retrieve the current entries ...
    let catalog_fname = &args.input_file;
    let mut catalog_content: AmsdosEntries = {
        let mut content = Vec::new();

        if catalog_fname.to_lowercase().contains("dsk") || catalog_fname.to_lowercase().contains("hfe") {
            // Read a dsk or hfe file
            error!(
                "Current implementation is buggy when using dsks. Please extract first the catalog with another tool for real results."
            );
            let disc = open_disc(catalog_fname, true).expect("unable to read the disc file");
            let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
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
    let catalog_fname_lower = catalog_fname.to_lowercase();
    if catalog_fname_lower.contains("dsk") || catalog_fname_lower.contains("hfe") {
        list_catalog_entries(&catalog_content, args.listall);
    }

    if let Some(idx) = args.entry {
        info!("Manipulate entry {idx}");

        let entry = catalog_content.get_entry_mut(idx as _);

        if args.setreadonly {
            entry.set_read_only();
        }
        if args.setsystem {
            entry.set_system();
        }
        if args.unsetreadonly {
            entry.unset_read_only();
        }
        if args.unsetsystem {
            entry.unset_system();
        }

        if let Some(user) = args.user {
            entry.set_user(user);
        }

        if let Some(ref filename) = args.filename {
            entry.set_filename(filename);
        }

        if let Some(ref blocs) = args.blocs {
            let blocs = blocs
                .iter()
                .map(|bloc| BlocIdx::from(*bloc))
                .collect::<Vec<BlocIdx>>();
            entry.set_blocs(&blocs);
        }

        if let Some(ref numpage) = args.numpage {
            entry.set_num_page(to_number::<u8>(numpage));
        }

        // XXX It is important ot keep it AFTER the blocs as it override their value
        if let Some(ref size) = args.size {
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
