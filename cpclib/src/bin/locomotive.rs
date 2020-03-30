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

///! Locomotive BASIC manipulation tool.
use clap;
use cpclib;

use cpclib::basic::BasicProgram;
use cpclib::disc::amsdos::AmsdosManager;

use clap::*;

use std::fs::File;
use std::io::{Read, Write};

fn main() -> std::io::Result<()> {
    let matches = App::new("locomotive")
        .about("Locomotive basic manipulation tool")
        .after_help("Krusty/Benediction 2019")
        .arg(
            Arg::with_name("BASIC_SOURCE")
                .long("basic")
                .short("b")
                .help("Source file that contains the basic program")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("HEADER")
                .long("header")
                .short("h")
                .help("Add the Amsdos header to the generated file"),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Output file")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    // Read the basic source file
    let basic_content: String = {
        let mut f = File::open(matches.value_of("BASIC_SOURCE").unwrap())?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        content
    };

    // Extract the basic tokens
    let basic_tokens = match BasicProgram::parse(basic_content) {
        Ok(tokens) => tokens,
        Err(msg) => panic!("Unable to parse Basic: {}", msg),
    };

    // Bytes of the basic program
    let basic_bytes = basic_tokens.as_bytes();

    if let Some(output) = matches.value_of("OUTPUT") {
        let mut f = File::create(output)?;

        // Add header if needed
        if matches.is_present("HEADER") {
            let header = AmsdosManager::compute_basic_header(&output.into(), &basic_bytes);
            f.write_all(header.as_bytes().as_ref())?;
        }

        // Add the tokens
        f.write_all(&basic_bytes)?;
    }

    Ok(())
}
