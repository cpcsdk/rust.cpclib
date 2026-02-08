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

use fs_err::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use cpclib_basic::{BasicProgram, binary_parser};
use cpclib_common::clap;
/// ! Locomotive BASIC manipulation tool.
use cpclib_common::clap::{Arg, ArgAction, Command};
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosHeader};

fn main() -> std::io::Result<()> {
    let matches = Command::new("locomotive")
        .about("Locomotive basic manipulation tool")
        .after_help("Krusty/Benediction 2019")
        .arg(
            Arg::new("BASIC_BINARY")
                .long("basic")
                .short('b')
                .help("Amstrad basic file")
                .action(ArgAction::Set)
                .value_parser(cpclib_common::utf8pathbuf_value_parser(true))
                .required_unless_present("BASIC_SOURCE")
        )
        .arg(
            Arg::new("BASIC_SOURCE")
                .long("ascii")
                .short('a')
                .help("Source file that contains the basic program as an ascii file")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(PathBuf))
                .required_unless_present("BASIC_BINARY")
        )
        .arg(
            Arg::new("HEADER")
                .long("header")
                .short('H')
                .help("Add the Amsdos header to the generated basic file or do not read the amsdos header of the basic file")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("OUTPUT")
                .help("Output file")
                .value_parser(clap::value_parser!(PathBuf))
                .action(ArgAction::Set) //         .required(true)
        )
        .get_matches();

    if let Some(fname) = matches.get_one::<PathBuf>("BASIC_SOURCE") {
        // Read the basic source file
        let basic_content: String = {
            let mut f = File::open(fname)?;
            let mut content = String::new();
            f.read_to_string(&mut content)?;
            content
        };

        // Extract the basic tokens
        let basic_tokens = match BasicProgram::parse(basic_content) {
            Ok(tokens) => tokens,
            Err(msg) => panic!("Unable to parse Basic: {msg}")
        };

        // Bytes of the basic program
        let basic_bytes = basic_tokens.as_bytes();

        if let Some(output) = matches.get_one::<PathBuf>("OUTPUT") {
            let mut f = File::create(output)?;

            // Add header if needed
            if matches.contains_id("HEADER") {
                let header = AmsdosHeader::compute_basic_header(
                    &AmsdosFileName::from_slice(output.display().to_string().as_bytes()),
                    &basic_bytes
                );
                f.write_all(header.as_bytes().as_ref())?;
            }

            f.write_all(&basic_bytes)?;
        }
    }
    else if let Some(fname) = matches.get_one::<PathBuf>("BASIC_BINARY") {
        // Read the basic source file
        let ascii_content: Vec<u8> = fs_err::read(fname)?;
        let mut ascii_content = &ascii_content[..];
        let tokens = binary_parser::program(&mut ascii_content).expect("Error in the basic file");

        dbg!(&tokens);

        todo!("print");
    }
    else {
        unreachable!()
    }

    Ok(())
}
