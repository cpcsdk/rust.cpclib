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
use std::io::Read;
use std::io::Write;
use cpclib_common::clap;
use cpclib_disc::amsdos::*;

/**
 * Convert a string to its unsigned 32 bits representation (to access to extra memory)
 * TODO share implementation
 */
pub fn string_to_nb(source: &str) -> u32 {
    let error = format!("Unable to read the value: {}", source);
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    } else {
        source.parse::<u32>().expect(&error)
    }
}

fn main() -> std::io::Result<()> {
    let matches = clap::App::new("hideur")
        .arg(
            clap::Arg::with_name("INPUT")
                .required(true)
                .help("Input file to manipulate"),
        )
        .arg(clap::Arg::with_name("INFO").long("info"))
        .arg(
            clap::Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .required_unless("INFO")
                .help("Output file to generate")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("USER")
                .short("u")
                .long("user")
                .conflicts_with("INFO")
                .help("User where to put the file")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("TYPE")
                .short("t")
                .long("type")
                .conflicts_with("INFO")
                .required_unless("INFO")
                .help("File type")
                .case_insensitive(true)
                .possible_values(&["0", "1", "2", "Basic", "Protected", "Binary"])
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("EXEC")
                .short("x")
                .long("execution")
                .conflicts_with("INFO")
                .help("Execution address")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("LOAD")
                .short("-l")
                .long("load")
                .conflicts_with("INFO")
                .help("Loading address")
                .takes_value(true),
        )
        .get_matches();

    // Read the input file
    let complete_filename = matches.value_of("INPUT").unwrap();

    let content = {
        let input = complete_filename;
        let mut f = File::open(input)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        buf
    };

    // Get filename and extension
    let filename = {
        let user = matches.value_of("USER").map_or(0, string_to_nb) as u8;
        let (filename, extension) = {
            let parts = complete_filename.split('.').collect::<Vec<_>>();
            let (filename, extension) = match parts.len() {
                1 => (parts[0].to_owned(), "".to_owned()),
                2 => (parts[0].to_owned(), parts[1].to_owned()),
                _n => {
                    eprintln!(
                        "[Warning] Filename contains several `.`. They have been all removed."
                    );
                    (
                        parts[..parts.len() - 1].join("_").to_owned(),
                        parts[parts.len() - 1].to_owned(),
                    )
                }
            };

            let filename = if filename.len() > 8 {
                eprintln!("[Warning] Filename is too large and has been cropped. If it is not the expected behavior provide a file with the right filename");
                filename[..8].to_owned()
            } else {
                filename
            };

            let extension = if extension.len() > 3 {
                eprintln!("[Warning] Extension is too large and has been cropped. If it is not the expected behavior provide a file with the right extension");
                extension[..3].to_owned()
            } else {
                extension
            };

            (filename, extension)
        };

        AmsdosFileName::new_correct_case(user, filename, extension)
            .expect("Invalid file definition")
    };

    if matches.is_present("INFO") {
        // In this branch we display information about the header
        let amsfile = AmsdosFile::from_buffer(&content);
        let header = amsfile.header();
        if header.is_checksum_valid() {
            println!("{:?}", header);
        } else {
            eprintln!("This is not an Amsdos file");
        }
    } else {
        // In this branch, we build the file with its header

        // Get the type of file
        let ftype = {
            match matches
                .value_of("TYPE")
                .unwrap()
                .to_ascii_lowercase()
                .as_ref()
            {
                "0" | "basic" => AmsdosFileType::Basic,
                "1" | "protected" => AmsdosFileType::Protected,
                "2" | "binary" => AmsdosFileType::Binary,
                _ => unreachable!(),
            }
        };

        // Build the header according to the given options
        let header = match ftype {
            AmsdosFileType::Binary => {
                let exec = string_to_nb(
                    &matches
                        .value_of("EXEC")
                        .expect("The execution address is expected for a binary target"),
                ) as u16;
                let load = string_to_nb(
                    &matches
                        .value_of("LOAD")
                        .expect("The load address is expected for a binary target"),
                ) as u16;

                AmsdosManager::compute_binary_header(&filename, load, exec, &content)
            }
            AmsdosFileType::Basic => AmsdosManager::compute_basic_header(&filename, &content),
            _ => unimplemented!(),
        };

        // Write the final file
        let mut f = File::create(matches.value_of("OUTPUT").unwrap())?;
        f.write_all(header.as_bytes())?;
        f.write_all(&content)?;
    }
    Ok(())
}
