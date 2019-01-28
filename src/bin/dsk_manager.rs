extern crate clap;
extern crate cpclib;

use std::fs::File;
use std::io::{Read, Write};

use clap::{Arg, App, SubCommand, ArgGroup};


use cpclib::disc::edsk::ExtendedDsk;
use cpclib::disc::amsdos::*;

// Still everything to do
fn main() -> std::io::Result<()> {
    let matches = App::new("dsk_manager")
                       .about("Manipulate DSK files")
                       .author("Krusty/Benediction")
                       .after_help("Pale buggy copy of an old Ramlaid's tool")
                       .arg(
                           Arg::with_name("DSK_FILE")
                            .help("DSK file to manipulate")
                            .required(true)
                            .index(1)
                       )
                       .subcommand(
                           SubCommand::with_name("format")
                            .about("Format a dsk")
                            .arg(
                                Arg::with_name("FORMAT_FILE")
                                    .help("Provide a file that describes the format of the disc")
                                    .long("description")
                                    .short("d")
                                    .takes_value(true)
                            )
                            .arg(
                                Arg::with_name("FORMAT_NAME")
                                    .help("Provide the name of a format that can be used")
                                    .short("f")
                                    .long("format")
                                    .takes_value(true)
                                    .possible_values(&["data", "data42"])
                            )
                            .group(
                                ArgGroup::with_name("command")
                                    .arg("FORMAT_FILE")
                                    .arg("FORMAT_NAME")
                                    .required(true)
                            )
                       )
                       .subcommand(
                           SubCommand::with_name("catalog")
                           .about("Manipulate the catalog. Can only works for DSK having a Track 0 compatible with Amsdos")
                           .arg(
                               Arg::with_name("IMPORT")
                                .help("Import an existing catalog in the dsk. All entries are thus erased")
                                .long("import")
                                .short("-i")
                                .takes_value(true)
                           )
                           .arg(
                               Arg::with_name("EXPORT")
                                .help("Export the catalog in a specific file")
                                .long("export")
                                .short("-e")
                                .takes_value(true)
                           )
                           .arg(
                               Arg::with_name("LIST")
                               .help("Display the catalog on screen")
                               .long("list")
                               .short("l")
                           )
                           .arg(
                               Arg::with_name("CATART")
                               .help("[unimplemented] Display the catart version")
                               .long("--catart")
                           )
                           .group(
                               ArgGroup::with_name("command")
                                .arg("IMPORT")
                                .arg("EXPORT")
                                .arg("LIST")
                                .arg("CATART")
                                .required(true)
                           )
                       )
                       .subcommand(
                           SubCommand::with_name("add")
                           .about("Add files in the disc in an Amsdos way")
                           .arg(
                               Arg::with_name("INPUT_FILES")
                                .help("The files to add. They MUST have a header")
                                .takes_value(true)
                                .multiple(true)
                                .required(true)
                            )
                            .arg(
                                Arg::with_name("SYSTEM")
                                .help("Indicates if the files are system files")
                            )
                            .arg(
                                Arg::with_name("READ_ONLY")
                                .help("Indicates if the files are read only")
                            )
                       )
                       .subcommand(
                           SubCommand::with_name("put")
                           .about("Add files in the disc in a sectorial way")
                           .arg(
                               Arg::with_name("TRACK")
                                .help("The track of interest")
                                .short("a")
                                .takes_value(true)
                                .required(true)
                           )
                           .arg(
                               Arg::with_name("SECTOR")
                                .help("The sector of interest")
                                .short("o")
                                .takes_value(true)
                                .required(true)
                           )
                           .arg(
                               Arg::with_name("SIDE")
                                .help("The side of interest")
                                .short("p")
                                .takes_value(true)
                                .required(true)
                           )
                           .arg(
                               Arg::with_name("Z80_EXPORT")
                               .help("The path to the z80 files that will contains all the import information")
                                .short("z")
                                .takes_value(true)
                                .required(false)
                           )
                           .arg(
                               Arg::with_name("FILES")
                               .help("The ordered list of files to import in the dsk")
                                .takes_value(true)
                                .multiple(true)
                                .required(true)
                                .last(true)
                           )
                       )
                       .get_matches();

    let dsk_fname = matches.value_of("DSK_FILE").unwrap();

    // Manipulate the catalog of a disc
    if let Some(sub) = matches.subcommand_matches("catalog") {
        let mut dsk = ExtendedDsk::open(dsk_fname)
                              .expect(&format!("Unable to open the file {}", dsk_fname));
        eprintln!("WIP - We assume side 0 is chosen");

        // Import the catalog from one file in one existing disc
        if let Some(fname) = sub.value_of("IMPORT") {
            let mut f = File::open(fname)?;
            let mut bytes = Vec::new();
            let size = f.read_to_end(&mut bytes)?;

            if size != 64*32 {
                eprintln!("Catalog size uses {} bytes wheras it should be {}", size, 64*32);
            }

            for idx in 0..4 {
                let mut sector = dsk.sector_mut(0, 0, idx + 0xc1).expect("Wrong format");
                let idx = idx as usize;
                sector.set_values(&bytes[idx*512..(idx+1)*512]);
            }

            dsk.save(dsk_fname)?;

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
        else if let Some(fname) = sub.value_of("EXPORT") {
            eprintln!("WIP - We assume the format of the Track 0 is similar to Amsdos one");

            let manager = AmsdosManager::new_from_disc(dsk, 0);
            let bytes = manager.catalog().as_bytes();
            let mut f = File::create(fname)?;
            f.write_all(&bytes)?;
        }
        else if sub.is_present("LIST") {
            let manager = AmsdosManager::new_from_disc(dsk, 0);
            let catalog = manager.catalog();
            let entries = catalog.visible_entries().collect::<Vec<_>>();
            /// TODO manage files instead of entries
            println!("Dsk {} -- {} files", dsk_fname, entries.len());
            for entry in entries.iter(){
                println!("{}", entry.to_string());
            }
        }
        else {
            panic!("Error - missing argument");
        }
        
    }

    // Add files in an Amsdos compatible disc
    else if let Some(sub) = matches.subcommand_matches("add") {

        // Get the input dsk
       let dsk = ExtendedDsk::open(dsk_fname)
                              .expect(&format!("Unable to open the file {}", dsk_fname));
        let mut manager = AmsdosManager::new_from_disc(dsk, 0);

        // Get the common parameters
        let is_system = sub.is_present("SYSTEM");
        let is_read_only = sub.is_present("READ_ONLY");

        // loop over all the files to add them
        for fname in matches.values_of("INPUT_FILES").unwrap() {
            let ams_file = match AmsdosFile::open_valid(fname) {
                Ok(ams_file) => {
                    println!("{:?} added", ams_file.amsdos_filename());
                    ams_file
                },
                Err(e) => {
                    panic!("Unable to load {}: {:?}", fname, e);
                }
            };

            manager.add_file(&ams_file, is_system, is_read_only);
        }

        // Save the dsk on disc
        manager.dsk().save(dsk_fname)?;
    }

    // Manage the formating of a disc
    else if let Some(sub) = matches.subcommand_matches("format") {
        use cpclib::disc::cfg::DiscConfig;

        // Retrieve the format description
        let cfg = if let Some(desc_fname) = sub.value_of("FORMAT_FILE") {
             cpclib::disc::cfg::DiscConfig::new(desc_fname)?
        }
        else if let Some(desc) = sub.value_of("FORMAT_NAME") {
            match desc {
                "data42" => DiscConfig::single_side_data42_format(),
                "data"   => DiscConfig::single_side_data_format(),
                _ =>  unreachable!()
            }
        }
        else {
            unreachable!();
        };

        // Make the dsk based on the format
        let dsk = cpclib::disc::builder::build_disc_from_cfg(&cfg);
        dsk.save(dsk_fname)?;
    }


    else {
        eprintln!("Missing command\n{}", matches.usage());
    }

    Ok(())
}
