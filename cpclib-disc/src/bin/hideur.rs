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
#![allow(clippy::cast_possible_truncation)]

/// (unfinished) conversion of hideur maker
use std::fs::File;
use std::io::{Read, Write};

use clap::value_parser;
use cpclib_common::clap::{self, ArgAction};
use cpclib_disc::amsdos::{AmsdosFile, AmsdosFileName, AmsdosFileType, AmsdosHeader};

/// Convert a string to its unsigned 32 bits representation (to access to extra memory)
/// TODO share implementation
#[must_use]
pub fn string_to_nb(source: &str) -> u32 {
    let error = format!("Unable to read the value: {source}");
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    }
    else {
        source.parse::<u32>().expect(&error)
    }
}

fn main() -> std::io::Result<()> {
    let matches = clap::Command::new("hideur")
        .arg(
            clap::Arg::new("INPUT")
                .required(true)
                .help("Input file to manipulate")
        )
        .arg(
            clap::Arg::new("INFO")
                .long("info")
                .action(ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("OUTPUT")
                .short('o')
                .long("output")
                .required_unless_present("INFO")
                .help("Output file to generate")
        )
        .arg(
            clap::Arg::new("USER")
                .short('u')
                .long("user")
                .conflicts_with("INFO")
                .help("User where to put the file")
                .value_parser(value_parser!(u8))
        )
        .arg(
            clap::Arg::new("TYPE")
                .short('t')
                .long("type")
                .conflicts_with("INFO")
                .required_unless_present("INFO")
                .help("File type")
                .ignore_case(true)
                .value_parser(["0", "1", "2", "Basic", "Protected", "Binary"])
        )
        .arg(
            clap::Arg::new("EXEC")
                .short('x')
                .long("execution")
                .conflicts_with("INFO")
                .help("Execution address")
        )
        .arg(
            clap::Arg::new("LOAD")
                .short('l')
                .long("load")
                .conflicts_with("INFO")
                .help("Loading address")
        )
        .get_matches();

    // Read the input file
    let complete_filename = matches.get_one::<String>("INPUT").unwrap();

    let content = {
        let input = complete_filename;
        let mut f = File::open(input)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        buf
    };

    // Get filename and extension
    let filename = {
        let user = matches.get_one::<u8>("USER").copied().unwrap_or(0);
        let (filename, extension) = {
            let parts = complete_filename.split('.').collect::<Vec<_>>();
            let (filename, extension) = match parts.len() {
                1 => (parts[0].to_owned(), String::new()),
                2 => (parts[0].to_owned(), parts[1].to_owned()),
                _n => {
                    eprintln!(
                        "[Warning] Filename contains several `.`. They have been all removed."
                    );
                    (
                        parts[..parts.len() - 1].join("_"),
                        parts[parts.len() - 1].to_owned()
                    )
                }
            };

            let filename = if filename.len() > 8 {
                eprintln!("[Warning] Filename is too large and has been cropped. If it is not the expected behavior provide a file with the right filename");
                filename[..8].to_owned()
            }
            else {
                filename
            };

            let extension = if extension.len() > 3 {
                eprintln!("[Warning] Extension is too large and has been cropped. If it is not the expected behavior provide a file with the right extension");
                extension[..3].to_owned()
            }
            else {
                extension
            };

            (filename, extension)
        };

        AmsdosFileName::new_correct_case(user, filename, extension)
            .expect("Invalid file definition")
    };

    if matches.get_flag("INFO") {
        // In this branch we display information about the header
        let amsfile = AmsdosFile::from_buffer(&content);
        match amsfile.header() {
            Some(header) => {
                println!("{header:?}");
            },
            None => {
                eprintln!("This is an ASCII file");
            }
        }
    }
    else {
        // In this branch, we build the file with its header

        // Get the type of file
        let ftype = {
            match matches
                .get_one::<String>("TYPE")
                .unwrap()
                .to_ascii_lowercase()
                .as_ref()
            {
                "0" | "basic" => AmsdosFileType::Basic,
                "1" | "protected" => AmsdosFileType::Protected,
                "2" | "binary" => AmsdosFileType::Binary,
                _ => unreachable!()
            }
        };

        // Build the header according to the given options
        let header = match ftype {
            AmsdosFileType::Binary => {
                let exec = string_to_nb(
                    matches
                        .get_one::<String>("EXEC")
                        .expect("The execution address is expected for a binary target")
                ) as u16;
                let load = string_to_nb(
                    matches
                        .get_one::<String>("LOAD")
                        .expect("The load address is expected for a binary target")
                ) as u16;

                AmsdosHeader::compute_binary_header(&filename, load, exec, &content)
            },
            AmsdosFileType::Basic => AmsdosHeader::compute_basic_header(&filename, &content),
            _ => unimplemented!()
        };

        // Write the final file
        let mut f = File::create(matches.get_one::<String>("OUTPUT").unwrap())?;
        f.write_all(header.as_bytes())?;
        f.write_all(&content)?;
    }
    Ok(())
}
