//#![feature(register_attr)]
//#![register_attr(get)]

use std::fs::File;
use std::io::Write;

use cpclib_common::clap::*;
use disc::Disc;
use edsk::Head;

/// Concerns all stuff related to Amsdos disc format
pub mod amsdos;
/// Utility function to build a DSK thanks to a format description
pub mod builder;
/// Parser of the format description
pub mod cfg;
pub mod disc;
/// EDSK File format
pub mod edsk;

#[cfg(hfe)]
/// HFE File format
pub mod hfe;

use std::io::Read;
use std::path::Path;
use std::str::FromStr;

use custom_error::custom_error;

use crate::amsdos::{AmsdosFile, AmsdosManagerMut, AmsdosManagerNonMut};
use crate::edsk::ExtendedDsk;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

custom_error! {pub DskManagerError
    IOError{source: std::io::Error} = "IO error: {source}.",
    AnyError{msg: String} = "{msg}",
    DiscConfigError{source: crate::cfg::DiscConfigError} = "Disc configuration: {source}",
}

pub fn dsk_manager_handle(matches: ArgMatches) -> Result<(), DskManagerError> {
    let dsk_fname = matches.get_one::<String>("DSK_FILE").unwrap();

    // Manipulate the catalog of a disc
    if let Some(sub) = matches.subcommand_matches("catalog") {
        let mut dsk = ExtendedDsk::open(dsk_fname)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));
        eprintln!("WIP - We assume head 0 is chosen");

        // Import the catalog from one file in one existing disc
        if let Some(fname) = sub.get_one::<String>("IMPORT") {
            let mut f = File::open(fname)?;
            let mut bytes = Vec::new();
            let size = f.read_to_end(&mut bytes)?;

            if size != 64 * 32 {
                eprintln!(
                    "Catalog size uses {} bytes whereas it should be {}",
                    size,
                    64 * 32
                );
            }

            for idx in 0..4 {
                let sector = dsk.sector_mut(0, 0, idx + 0xc1).expect("Wrong format");
                let idx = idx as usize;
                sector
                    .set_values(&bytes[idx * 512..(idx + 1) * 512])
                    .unwrap();
            }

            dsk.save(dsk_fname)
            .map_err(|e| DskManagerError::AnyError { msg: e })?;

        /*
            // TODO find why this method DOES NOT WORK
                // Generate the entry for this catart
               let  entries = AmsdosEntries::from_slice(&bytes);

               // And inject it in the disc
               let mut manager = AmsdosManager::new_from_disc(dsk, 0);
               manager.set_catalog(&entries);

               let copy = manager.catalog();
               assert_eq!(
                   copy,
                   entries
               );
               manager.dsk_mut().save(dsk_fname)?;
        */
        // override the disc
        }
        // Export the catalog of an existing disc in a file
        else if let Some(fname) = sub.get_one::<String>("EXPORT") {
            eprintln!("WIP - We assume the format of the Track 0 is similar to Amsdos one");

            let manager = AmsdosManagerNonMut::new_from_disc(&mut dsk, 0);
            let bytes = manager.catalog().as_bytes();
            let mut f = File::create(fname)?;
            f.write_all(&bytes)?;
        } else if sub.contains_id("LIST") {
            let manager = AmsdosManagerNonMut::new_from_disc(&mut dsk, 0);
            let catalog = manager.catalog();
            let entries = catalog.visible_entries().collect::<Vec<_>>();
            // TODO manage files instead of entries
            println!("Dsk {} -- {} files", dsk_fname, entries.len());
            for entry in &entries {
                println!("{}", entry);
            }
        } else {
            panic!("Error - missing argument");
        }
    }
    else if let Some(sub) = matches.subcommand_matches("put") {
        use cpclib_tokens::{builder, Listing};

        // Add files in a sectorial way
        let mut track =
            u8::from_str(sub.get_one::<String>("TRACK").unwrap()).expect("Wrong track format");
        let mut sector =
            u8::from_str(sub.get_one::<String>("SECTOR").unwrap()).expect("Wrong track format");
        let mut head =
            u8::from_str(sub.get_one::<String>("SIDE").unwrap()).expect("Wrong track format");
        let _export = sub.get_one::<String>("Z80_EXPORT").unwrap();

        let mut dsk = ExtendedDsk::open(dsk_fname)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));

        let mut listing = Listing::new();
        for file in sub.get_many::<String>("FILES").unwrap() {
            // get the file
            let mut f = File::open(file)?;
            let mut content = Vec::new();
            f.read_to_end(&mut content)?;

            let next_position = dsk
                .add_file_sequentially(head, track, sector, &content)
                .unwrap_or_else(|_| panic!("Unable to add {file}"));

            let base_label = Path::new(file)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('.', "_");
            listing.add(builder::equ(format!("{}_head", &base_label), head));
            listing.add(builder::equ(format!("{}_track", &base_label), track));
            listing.add(builder::equ(format!("{}_sector", &base_label), sector));

            head = next_position.0;
            track = next_position.1;
            sector = next_position.2;
        }
    }
    else if let Some(sub) = matches.subcommand_matches("add") {
        // Add files in an Amsdos compatible disc

        // Get the input dsk
        let mut dsk = ExtendedDsk::open(dsk_fname)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));

        // Get the common parameters
        let is_system = sub.contains_id("SYSTEM");
        let is_read_only = sub.contains_id("READ_ONLY");
        let head = Head::A;

        // loop over all the files to add them
        for fname in sub.get_many::<String>("INPUT_FILES").unwrap() {
            let ams_file = match AmsdosFile::open_valid(fname) {
                Ok(ams_file) => {
                    assert!(
                        ams_file.amsdos_filename().unwrap().is_valid(),
                        "Invalid amsdos filename ! {:?}",
                        ams_file.amsdos_filename()
                    );
                    println!("{:?} added", ams_file.amsdos_filename());
                    ams_file
                },
                Err(e) => {
                    panic!("Unable to load {fname}: {e:?}");
                }
            };
            dsk.add_amsdos_file(&ams_file, head, is_system, is_read_only)
                .unwrap();
        }

        // Save the dsk on disc
        dsk.save(dsk_fname)
            .map_err(|e| DskManagerError::AnyError { msg: e })?;
    }
    else if let Some(sub) = matches.subcommand_matches("format") {
        // Manage the formating of a disc
        use crate::cfg::DiscConfig;

        // Retrieve the format description
        let cfg = if let Some(desc_fname) = sub.get_one::<String>("FORMAT_FILE") {
            crate::cfg::DiscConfig::new(desc_fname)?
        }
        else if let Some(desc) = sub.get_one::<String>("FORMAT_NAME") {
            match desc.as_str() {
                "data42" => DiscConfig::single_head_data42_format(),
                "data" => DiscConfig::single_head_data_format(),
                _ => unreachable!()
            }
        }
        else {
            unreachable!();
        };

        // Make the dsk based on the format
        let dsk = crate::builder::build_edsk_from_cfg(&cfg);
        dsk.save(dsk_fname)
            .map_err(|e| DskManagerError::AnyError { msg: e })?;
    }
    else {
        eprintln!("Missing command\n");
    }

    Ok(())
}

pub fn dsk_manager_build_arg_parser() -> Command {
    Command::new("dsk_manager")
                       .about("Manipulate DSK files")
                       .author("Krusty/Benediction")
                       .after_help("Pale buggy copy of an old Ramlaid's tool")
                       .arg(
                           Arg::new("DSK_FILE")
                            .help("DSK file to manipulate")
                            .required(true)
                            .index(1)
                       )
                       .subcommand(
                           Command::new("format")
                            .about("Format a dsk")
                            .arg(
                                Arg::new("FORMAT_FILE")
                                    .help("Provide a file that describes the format of the disc")
                                    .long("description")
                                    .short('d')
                            )
                            .arg(
                                Arg::new("FORMAT_NAME")
                                    .help("Provide the name of a format that can be used")
                                    .short('f')
                                    .long("format")
                                    .value_parser(["data", "data42"])
                            )
                            .group(
                                ArgGroup::new("command")
                                    .arg("FORMAT_FILE")
                                    .arg("FORMAT_NAME")
                            )
                       )
                       .subcommand(
                           Command::new("catalog")
                           .about("Manipulate the catalog. Can only works for DSK having a Track 0 compatible with Amsdos")
                           .arg(
                               Arg::new("IMPORT")
                                .help("Import an existing catalog in the dsk. All entries are thus erased")
                                .long("import")
                                .short('i')
                           )
                           .arg(
                               Arg::new("EXPORT")
                                .help("Export the catalog in a specific file")
                                .long("export")
                                .short('e')
                           )
                           .arg(
                               Arg::new("LIST")
                               .help("Display the catalog on screen")
                               .long("list")
                               .short('l')
                        .action(ArgAction::SetTrue)

                           )
                           .arg(
                               Arg::new("CATART")
                               .help("[unimplemented] Display the catart version")
                               .long("--catart")
                        .action(ArgAction::SetTrue)

                           )
                           .group(
                               ArgGroup::new("command")
                                .arg("IMPORT")
                                .arg("EXPORT")
                                .arg("LIST")
                                .arg("CATART")
                                .required(true)
                           )
                       )
                       .subcommand(
                           Command::new("add")
                           .about("Add files in the disc in an Amsdos way")
                           .arg(
                               Arg::new("INPUT_FILES")
                                .help("The files to add. They MUST have a header")
                                .action(ArgAction::Append)
                                .required(true)
                            )
                            .arg(
                                Arg::new("SYSTEM")
                                .help("Indicates if the files are system files")
                                .long("system")
                                .short('s')
                        .action(ArgAction::SetTrue)

                            )
                            .arg(
                                Arg::new("READ_ONLY")
                                .help("Indicates if the files are read only")
                                .long("read_only")
                                .short('r')
                        .action(ArgAction::SetTrue)

                            )
                            .arg(
                                Arg::new("AS_AMSDOS")
                                .help("[unimplemented] Uses the same strategy as amsdos when adding a file: add .???, delete .BAK, rename other as .BAK, rename .??? with real extension")
                                .long("secure")
                        .action(ArgAction::SetTrue)

                            )
                       )
                       .subcommand(
                           Command::new("put")
                           .about("Add files in the disc in a sectorial way")
                           .arg(
                               Arg::new("TRACK")
                                .help("The track of interest")
                                .short('a')
                                .required(true)
                           )
                           .arg(
                               Arg::new("SECTOR")
                                .help("The sector of interest")
                                .short('o')
                                .required(true)
                           )
                           .arg(
                               Arg::new("SIDE")
                                .help("The head of interest")
                                .short('p')
                                .required(true)
                           )
                           .arg(
                               Arg::new("Z80_EXPORT")
                               .help("The path to the z80 files that will contains all the import information")
                                .short('z')
                                .required(false)
                           )
                           .arg(
                               Arg::new("FILES")
                               .help("The ordered list of files to import in the dsk")
                               .action(ArgAction::Append)
                                .required(true)
                                .last(true)
                           )
                       )
}
