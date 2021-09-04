use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use std::rc::Rc;

use crate::preamble::*;
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosManager};

use clap;
use clap::{App, Arg, ArgGroup, ArgMatches};
use itertools::chain;


#[derive(Debug)]
pub enum BasmError {
    //#[fail(display = "IO error: {}", io)]
    Io { io: io::Error, ctx: String },

    // #[fail(display = "Assembling error: {}", error)]
    AssemblerError { error: AssemblerError },

    // #[fail(display = "Invalid Amsdos filename: {}", filename)]
    InvalidAmsdosFilename { filename: String },

    // #[fail(display = "{} is not a valid directory.", path)]
    NotAValidDirectory { path: String },

    //  #[fail(display = "{} is not a valid file.", file)]
    NotAValidFile { file: String },

    ListingGeneration { msg: String}
}

impl Display for BasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {


        match self {
            BasmError::Io { io, ctx } => write!(f, "IO Error when {}: {}", ctx, io),
            BasmError::AssemblerError { error } => write!(f, "Assembling error:\n{}", error),
            BasmError::InvalidAmsdosFilename { filename } => {
                write!(f, "Invalid Amsdos filename: {}", filename)
            }
            BasmError::NotAValidDirectory { path } => {
                write!(f, "{} is not a valid directory.", path)
            }
            BasmError::NotAValidFile { file } => write!(f, "{} is not a valid file.", file),

            BasmError::ListingGeneration { msg } => write!(
                f, "Error when generating the symbol table: {}", msg
            ),
        }
    }
}


impl From<AssemblerError> for BasmError {
    fn from(error: AssemblerError) -> Self {
        BasmError::AssemblerError { error }
    }
}

/// Parse the given code.
/// TODO read options to configure the search path
pub fn parse<'arg>(matches: &'arg ArgMatches<'_>) -> Result<(LocatedListing, Vec<AssemblerError>), BasmError> {
    let (filename, code) = {
        if let Some(filename) = matches.value_of("INPUT") {
            let mut f = File::open(filename)
                        .map_err(|e| {
                            BasmError::Io {
                                io: e,
                                ctx: format!("opening \"{}\"", filename)
                            }
                        })?;

            // Rust is boring for non utf8? that is why we have to play with that
            let mut content = Vec::new();
            f.read_to_end(&mut content).map_err(|e| {
                BasmError::Io {
                    io: e,
                    ctx: format!("reading \"{}\"", filename)
                }
            })?;

            let result = chardet::detect(&content);
            let coder = encoding::label::encoding_from_whatwg_label(
                chardet::charset2encoding(&result.0),
            );

            let content = match coder {
                Some(coder) => {
                    let utf8reader = coder
                        .decode(&content, encoding::DecoderTrap::Ignore)
                        .expect("Error");
                    utf8reader.to_string()
                }
                None => {
                    return Err(AssemblerError::IOError {
                        msg: format!("Encoding error for {:?}.", filename),
                    }.into());
                }
            };

            (filename, content)
        } else if let Some(code) = matches.value_of("INLINE") {
            ("<inline code>", format!(" {}", code))
        } else {
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

    let code = Rc::new(code);
    let context = Rc::new(context);

    let res = parse_z80_strrc_with_contextrc(code, Rc::clone(&context))
    .map_err(|e| BasmError::from(e))?;

    let warnings = context.warnings();
    Ok((res, warnings))
}

/// Assemble the given code
/// TODO use options to configure the base symbole table
pub fn assemble<'arg>(
    matches: &'arg ArgMatches<'_>,
    listing: &LocatedListing
) -> Result<Env, BasmError> {
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

    if let Some(dest) = matches.value_of("LISTING_OUTPUT") {
        if dest == "-" {
            options.write_listing_output(std::io::stdout());
        } else {
            let file = File::create(dest).map_err(|e| {
                BasmError::Io{io: e, ctx: format!("creating {}", dest)}
                })?;
            options.write_listing_output(file);
        }
    }



    let env = visit_tokens_all_passes_with_options(&listing, &options)
        .map_err(|e| BasmError::AssemblerError{error: e})?;

    if let Some(dest) = matches.value_of("SYMBOLS_OUTPUT") {
        if dest == "-" {
            env.generate_symbols_output(&mut std::io::stdout())
        } else {
            let mut f = File::create(dest).map_err(|e| {
                BasmError::Io{io: e, ctx: format!("creating {}", dest)}
                })?;
                env.generate_symbols_output(&mut f)
        }.map_err(|err| {
                BasmError::ListingGeneration{msg: err.to_string()}
            })?;
    }

    Ok(env)
}

/// Save the provided result
/// TODO manage the various save options
pub fn save(matches: &ArgMatches<'_>, env: &Env) -> Result<(), BasmError> {
    if matches.is_present("SNAPSHOT") {
        let pc_filename = matches.value_of("OUTPUT").unwrap();
        env.save_sna(pc_filename).map_err(|e| {
            BasmError::Io {
                io: e,
                ctx: format!("saving \"{}\"", pc_filename)
            }
        })?;
    } else if matches.is_present("OUTPUT") {
        // Collect the produced bytes
        let binary = env.produced_bytes();

        if matches.is_present("DB_LIST") {
            let bytes = env.produced_bytes();
            if !bytes.is_empty() {
                let listing = Listing::from(bytes.as_ref());
                println!("{}", PrintableListing::from(&Listing::from(listing)));
            }
        } else {
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
                    env.loading_address().unwrap(),
                    env.execution_address().unwrap(),
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
            let mut f = File::create(pc_filename).map_err(|e| {
                BasmError::Io {
                    io: e,
                    ctx: format!("creating \"{}\"", pc_filename)
                }
            })?;
            if !header.is_empty() {
                f.write_all(&header)
                .map_err(|e| {
                    BasmError::Io {
                        io: e,
                        ctx: format!("saving \"{}\"", pc_filename)
                    }
                })?;
            }
            f.write_all(&binary)
                .map_err(|e| {
                    BasmError::Io {
                        io: e,
                        ctx: format!("saving \"{}\"", pc_filename)
                    }
                })?;
        }
    }
    Ok(())
}

/// Launch the assembling of everythin
pub fn process(matches: &ArgMatches<'_>) -> Result<(), BasmError> {
    // standard assembling
    let (listing, parser_warnings) = parse(matches)?;
    let env = assemble(matches, &listing)?;

    let assembler_warnings = env.warnings();
    if !assembler_warnings.is_empty() || !parser_warnings.is_empty() {
        for warning in chain!(parser_warnings.iter(), assembler_warnings.iter()){
            eprintln!("{}", warning);
        }

        if matches.is_present("WERROR") {
            eprintln!("Assembling failed due to unwanted warnings");
            std::process::exit(-1);
        }
    }
    save(matches, &env)
}
