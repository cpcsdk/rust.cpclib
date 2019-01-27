extern crate clap;
extern crate cpclib;

use std::fs::File;
use std::io::{Read, Write};

use clap::{Arg, App, SubCommand};


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
                       )
                       .subcommand(
                           SubCommand::with_name("format")
                            .about("Format a dsk")
                            .arg(
                                Arg::with_name("FORMAT_DESCRIPTION")
                                    .help("Provide a file that describes the format of the disc")
                                    .long("description")
                                    .short("d")
                                    .takes_value(true)
                            )
                       )
                       .subcommand(
                           SubCommand::with_name("catalog")
                           .about("Manipulate the catalog. Can only works for DSK having a Track 0 compatible with Amsdos")
                           .arg(
                               Arg::with_name("IMPORT")
                                .help("Import an existing catalog in the dsk. All entries are thus erased")
                                .long("import")
                                .takes_value(true)
                                .conflicts_with("EXPORT")
                                .conflicts_with("LIST")
                           )
                           .arg(
                               Arg::with_name("EXPORT")
                                .help("Export the catalog in a specific file")
                                .long("export")
                                .takes_value(true)
                                .conflicts_with("LIST")
                           )
                           .arg(
                               Arg::with_name("LIST")
                               .help("Display the catalog on screen")
                               .long("list")
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

    if let Some(sub) = matches.subcommand_matches("catalog") {
        let dsk = ExtendedDsk::open(dsk_fname)
                              .expect(&format!("Unable to open the file {}", dsk_fname));
        eprintln!("WIP - We assume side 0 is chosen");

        if let Some(fname) = sub.value_of("IMPORT") {
            let mut f = File::open(fname)?;
            let mut bytes = Vec::new();
            let size = f.read_to_end(&mut bytes)?;

            if size != 64*32 {
                eprintln!("Catalog size uses {} bytes wheras it should be {}", size, 64*32);
            }
           let  entries = AmsdosEntries::from_slice(&bytes);
           let mut manager = AmsdosManager::new_from_disc(dsk, 0);
           manager.set_catalog(&entries);
        }
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
            println!("Dsk {}", dsk_fname);
            for entry in entries.iter(){
                println!("{}", entry.to_string());
            }
        }
        else {
            panic!("Error - missing argument");
        }
        
    }

    else if let Some(sub) = matches.subcommand_matches("format") {
        if let Some(desc_fname) = sub.value_of("FORMAT_DESCRIPTION") {
            let cfg = cpclib::disc::cfg::DiscConfig::new(desc_fname)?;
            let dsk = cpclib::disc::builder::build_disc_from_cfg(&cfg);
            dsk.save(dsk_fname)?;
        }
    }


    Ok(())
}
