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

use std::fs::File;
use std::io;

use std::io::{Read, Write};
use std::path::Path;

use cpclib_asm::preamble::*;
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosManager};

use clap;
use clap::{App, Arg, ArgGroup, ArgMatches};
use failure::Fail;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug, Fail)]
enum BasmError {
    #[fail(display = "IO error: {}", io)]
    Io { io: io::Error },

    #[fail(display = "Assembling error: {}", error)]
    AssemblerError { error: AssemblerError },

    #[fail(display = "Invalid Amsdos filename: {}", filename)]
    InvalidAmsdosFilename { filename: String },

    #[fail(display = "{} is not a valid directory.", path)]
    NotAValidDirectory { path: String },

    #[fail(display = "{} is not a valid file.", file)]
    NotAValidFile { file: String },
}

// XXX I do not understand why I have to do that !!!
impl From<std::io::Error> for BasmError {
    fn from(error: std::io::Error) -> Self {
        BasmError::Io { io: error }
    }
}

impl From<AssemblerError> for BasmError {
    fn from(error: AssemblerError) -> Self {
        BasmError::AssemblerError { error }
    }
}

/// Parse the given code.
/// TODO read options to configure the search path
fn parse(matches: &ArgMatches<'_>) -> Result<Listing, BasmError> {
    

    let (filename, code) = {
        if let Some(filename) = matches.value_of("INPUT") {
            let mut f = File::open(filename)?;
            let mut content = String::new();
            f.read_to_string(&mut content)?;
            (filename, content)
        }
        else if let Some(code) = matches.value_of("INLINE") {
            ("<inline code>", format!(" {}", code))
        }
        else {
            panic!("No code provided to assemble");
        }
    };

    let mut context = ParserContext::default();
    context.set_current_filename(&filename);
    context.add_search_path_from_file(&filename)?;
    if let Some(directories) = matches.values_of("INCLUDE_DIRECTORIES") {
        for directory in directories {
            if !Path::new(directory).is_dir() {
                return Err(BasmError::NotAValidDirectory {
                    path: directory.to_owned(),
                });
            }
            context.add_search_path(directory)?;
        }
    }

    parse_str_with_context(&code, &context).map_err(|e| e.into())
}

/// Assemble the given code
/// TODO use options to configure the base symbole table
fn assemble(matches: &ArgMatches<'_>, listing: &Listing) -> Result<Env, BasmError> {
    let mut options = AssemblingOptions::default();

    options.set_case_sensitive(!matches.is_present("CASE_INSENSITIVE"));

    // TODO add symbols if any
    if let Some(files) = matches.values_of("LOAD_SYMBOLS") {
        for file in files {
            if !Path::new(file).is_file() {
                return Err(BasmError::NotAValidFile {
                    file: file.to_owned(),
                });
            }
        }
    }

    visit_tokens_all_passes_with_options(&listing.listing(), &options)
        .map_err(|e| e.into())
}

/// Save the provided result
/// TODO manage the various save options
fn save(matches: &ArgMatches<'_>, env: &Env) -> Result<(), BasmError> {
    if matches.is_present("SNAPSHOT") {
        let pc_filename = matches.value_of("OUTPUT").unwrap();
        env.save_sna(pc_filename)?;
    }
    else {
        // Collect the produced bytes
        let binary = env.produced_bytes();

        if matches.is_present("DB_LIST") {
            println!("{}", PrintableListing::from(&Listing::from(env.produced_bytes().as_ref())));
        }
        else {
            use std::convert::TryFrom;


            let pc_filename = matches.value_of("OUTPUT").unwrap();
            let amsdos_filename = AmsdosFileName::try_from(pc_filename);

            // Raise an error if the filename is not compatible with the header
            if matches.is_present("HEADER") && amsdos_filename.is_err() {
                return Err(BasmError::InvalidAmsdosFilename {
                    filename: pc_filename.to_string(),
                });
            }


            // Compute the headers if needed
            let header = if matches.is_present("BINARY_HEADER") {
                AmsdosManager::compute_binary_header(
                    &amsdos_filename.unwrap(),
                    env.loading_address().unwrap() as u16,
                    env.execution_address().unwrap() as u16,
                    &binary,
                )
                .as_bytes()
                .to_vec()
            } else if matches.is_present("BASIC_HEADER") {
                AmsdosManager::compute_basic_header(&amsdos_filename.unwrap(), &binary)
                    .as_bytes()
                    .to_vec()
            } else {
                Vec::new()
            };

            // Save file on disc
            let mut f = File::create(pc_filename)?;
            if !header.is_empty() {
                f.write_all(&header)?;
            }
            f.write_all(&binary)?;
        }

    }
    Ok(())
}

/// Launch the assembling of everythin
fn process(matches: &ArgMatches<'_>) -> Result<(), BasmError> {

    // standard assembling
    let listing = parse(matches)?;
    let env = assemble(matches, &listing)?;
    save(matches, &env)
}

fn main() {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );
    let matches = App::new("basm")
					.version(built_info::PKG_VERSION)
					.author("Krusty/Benediction")
					.about("Benediction ASM -- z80 assembler that taylor Amstrad CPC")
					.before_help(&desc_before[..])
                    .after_help("Work In Progress")
                    .arg(
                        Arg::with_name("INLINE")
                            .help("Z80 code is provided inline")
                            .long("inline")
                            .takes_value(true)
                    )
                    .arg(
						Arg::with_name("INPUT")
							.help("Input file to read.")
							.takes_value(true)
                    )
                    .group(
                        ArgGroup::with_name("ANY_INPUT")
                            .args(&["INLINE", "INPUT"])
                            .required(true)
                    )
					.arg(
						Arg::with_name("OUTPUT")
							.help("Filename of the output.")
							.short("o")
							.long("output")
							.takes_value(true)
					)
					.arg(
                        Arg::with_name("DB_LIST")
                        .help("Write a db list on screen (usefull to get the value of an opcode)")
                        .long("db")
                    )
                    .group(
                        ArgGroup::with_name("ANY_OUTPUT")
                            .args(&["DB_LIST", "OUTPUT"])
                            .required(true)
                    )
					.arg(
						Arg::with_name("BASIC_HEADER")
							.help("Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive.")
							.long("basic")
							.alias("basicheader")
					)
					.arg(
						Arg::with_name("BINARY_HEADER")
							.help("Request a binary header")
							.long("binary")
							.alias("header")
							.alias("binaryheader")
                    )
                    .arg(
                        Arg::with_name("SNAPSHOT")
                            .help("Generate a snapshot")
                            .long("snapshot")
                            .alias("sna")
                    )
					.arg(
						Arg::with_name("CASE_INSENSITIVE")
							.help("Configure the assembler to be case insensitive.")
							.long("case-insensitive")
							.short("i") 
					)
                    .arg(
                        Arg::with_name("INCLUDE_DIRECTORIES")
                            .help("Provide additional directories used to search files.")
                            .long("include")
                            .short("I")
                            .takes_value(true)
                            .multiple(true)
                            .number_of_values(1)
                    )
                    .arg(
                        Arg::with_name("LOAD_SYMBOLS")
                            .help("Load symbols from the given file")
                            .short("-l")
                            .takes_value(true)
                            .multiple(true)
                            .number_of_values(1)
                    )
					.group( // only one type of header can be provided
						ArgGroup::with_name("HEADER")
							.args(&["BINARY_HEADER", "BASIC_HEADER"])
                    )
                    .group( // only one type of output can be provided
                        ArgGroup::with_name("ARTEFACT_TYPE")
                        .args(&["BINARY_HEADER", "BASIC_HEADER", "SNAPSHOT"])
                    )
					.get_matches();

    match process(&matches) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Error while assembling.\n{}", e);
        }
    }
}

