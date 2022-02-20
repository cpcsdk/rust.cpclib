use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use cpclib_common::clap;
use cpclib_common::clap::{App, Arg, ArgGroup, ArgMatches};
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosManager};
use crate::processed_token::read_source;
use crate::preamble::*;

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

    ListingGeneration { msg: String },

    InvalidArgument(String)
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

            BasmError::ListingGeneration { msg } => {
                write!(f, "Error when generating the symbol table: {}", msg)
            }
            BasmError::InvalidArgument(msg) => {
                write!(f, "Invalid argument: {}", msg)
            }
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
pub fn parse<'arg>(
    matches: &'arg ArgMatches
) -> Result<(ParserContext, LocatedListing, Vec<AssemblerError>), BasmError> {
    let inline_fname = "<inline code>";
    let filename = matches.value_of("INPUT").unwrap_or(inline_fname);

    // prepare the context for the included directories
    let mut context = ParserContext::default();
    context.set_dotted_directives(matches.is_present("DOTTED_DIRECTIVES"));

    context.set_current_filename(&filename);
    context.add_search_path_from_file(&filename); // we ignore the potential error
    if let Some(directories) = matches.values_of("INCLUDE_DIRECTORIES") {
        for directory in directories {
            if !Path::new(directory).is_dir() {
                return Err(BasmError::NotAValidDirectory {
                    path: directory.to_owned()
                });
            }
            context.add_search_path(directory)?;
        }
    }

    // get the source code if any
    let code = if matches.is_present("INPUT") {
        read_source(filename, &context)?
    }
    else if let Some(code) = matches.value_of("INLINE") {
        format!(" {}", code)
    }
    else {
        panic!("No code provided to assemble");
    };

    // Continue the creation of the context
    let code = Arc::new(code);
    let context = Arc::new(context);

    let res = parse_z80_strrc_with_contextrc(code, Arc::clone(&context))
        .map_err(|e| BasmError::from(e))?;

    let warnings = context.warnings();
    Ok((context.as_ref().clone(), res, warnings))
}

/// Assemble the given code
/// TODO use options to configure the base symbole table
pub fn assemble<'arg>(
    matches: &'arg ArgMatches,
    ctx: &ParserContext,
    listing: &LocatedListing
) -> Result<Env, BasmError> {
    let mut options = AssemblingOptions::default();
    options.set_case_sensitive(!matches.is_present("CASE_INSENSITIVE"));

    // TODO add symbols if any
    if let Some(files) = matches.values_of("LOAD_SYMBOLS") {
        for file in files {
            if !Path::new(file).is_file() {
                return Err(BasmError::NotAValidFile {
                    file: file.to_owned()
                });
            }
        }
    }

    // Get the variables definition
    if let Some(definitions) = matches.values_of("DEFINE_SYMBOL") {
        for definition in definitions {
            let mut split = definition.split("=");
            let symbol = split.next().unwrap();
            let value = split.next().unwrap_or("1");
            let value = /*cpclib_common::*/parse_value(value.into())
                    .map_err(|_e| BasmError::InvalidArgument(definition.to_string()))
                    ?
                    .1;

            options
                .symbols_mut()
                .assign_symbol_to_value(symbol, value.eval()?)
                .map_err(|_e| BasmError::InvalidArgument(definition.to_string()))?;
        }
    }

    if let Some(dest) = matches.value_of("LISTING_OUTPUT") {
        if dest == "-" {
            options.write_listing_output(std::io::stdout());
        }
        else {
            let file = File::create(dest).map_err(|e| {
                BasmError::Io {
                    io: e,
                    ctx: format!("creating {}", dest)
                }
            })?;
            options.write_listing_output(file);
        }
    }

    let env = visit_tokens_all_passes_with_options(&listing, &options, &ctx)
        .map_err(|e| BasmError::AssemblerError { error: e })?;

    if let Some(dest) = matches.value_of("SYMBOLS_OUTPUT") {
        if dest == "-" {
            env.generate_symbols_output(&mut std::io::stdout())
        }
        else {
            let mut f = File::create(dest).map_err(|e| {
                BasmError::Io {
                    io: e,
                    ctx: format!("creating {}", dest)
                }
            })?;
            env.generate_symbols_output(&mut f)
        }
        .map_err(|err| {
            BasmError::ListingGeneration {
                msg: err.to_string()
            }
        })?;
    }

    Ok(env)
}

/// Save the provided result
/// TODO manage the various save options and delegate them with save commands
pub fn save(matches: &ArgMatches, env: &Env) -> Result<(), BasmError> {
    if matches.is_present("SNAPSHOT") {
        let pc_filename = matches.value_of("OUTPUT").unwrap();
        env.save_sna(pc_filename).map_err(|e| {
            BasmError::Io {
                io: e,
                ctx: format!("saving \"{}\"", pc_filename)
            }
        })?;
    }
    else if matches.is_present("OUTPUT") || matches.is_present("DB_LIST") {
        // Collect the produced bytes
        let binary = env.produced_bytes();

        if matches.is_present("DB_LIST") {
            let bytes = env.produced_bytes();
            if !bytes.is_empty() {
                let listing = Listing::from(bytes.as_ref());
                println!("{}", PrintableListing::from(&Listing::from(listing)));
            }
        }
        else {
            use std::convert::TryFrom;

            let pc_filename = matches.value_of("OUTPUT").unwrap();
            if pc_filename.to_lowercase().ends_with(".sna") && !matches.is_present("SNAPSHOT") {
                eprintln!(
                    "[WARNING] You are saving a file with .sna extension without using --sna flag"
                );
            }
            let amsdos_filename = AmsdosFileName::try_from(pc_filename);

            // Raise an error if the filename is not compatible with the header
            if matches.is_present("HEADER") && amsdos_filename.is_err() {
                return Err(BasmError::InvalidAmsdosFilename {
                    filename: pc_filename.to_string()
                });
            }

            // Compute the headers if needed
            let header = if matches.is_present("BINARY_HEADER") {
                AmsdosManager::compute_binary_header(
                    &amsdos_filename.unwrap(),
                    env.loading_address().unwrap(),
                    env.execution_address().unwrap(),
                    &binary
                )
                .as_bytes()
                .to_vec()
            }
            else if matches.is_present("BASIC_HEADER") {
                AmsdosManager::compute_basic_header(&amsdos_filename.unwrap(), &binary)
                    .as_bytes()
                    .to_vec()
            }
            else {
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
                f.write_all(&header).map_err(|e| {
                    BasmError::Io {
                        io: e,
                        ctx: format!("saving \"{}\"", pc_filename)
                    }
                })?;
            }
            f.write_all(&binary).map_err(|e| {
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
pub fn process(matches: &ArgMatches) -> Result<(Env, Vec<AssemblerError>), BasmError> {
    // standard assembling
    let (ctx, listing, mut warnings) = parse(matches)?;
    let env = assemble(matches, &ctx, &listing)?;

    warnings.extend_from_slice(env.warnings());

    if matches.is_present("WERROR") && !warnings.is_empty() {
        return Err(AssemblerError::MultipleErrors { errors: warnings }.into());
    }
    else {
        save(matches, &env)?;
        return Ok((env, warnings));
    }
}

pub fn build_args_parser() -> clap::App<'static> {
    App::new("basm")
					.author("Krusty/Benediction")
					.about("Benediction ASM -- z80 assembler that tailor Amstrad CPC")
                    .after_help("Work In Progress")
                    .arg(
                        Arg::new("INLINE")
                            .help("Z80 code is provided inline")
                            .long("inline")
                            .takes_value(true)
                    )
                    .arg(
						Arg::new("INPUT")
							.help("Input file to read.")
							.takes_value(true)
                    )
                    .group(
                        ArgGroup::new("ANY_INPUT")
                            .args(&["INLINE", "INPUT"])
                            .required(true)
                    )
					.arg(
						Arg::new("OUTPUT")
							.help("Filename of the output.")
							.short('o')
							.long("output")
							.takes_value(true)
					)
					.arg(
                        Arg::new("DB_LIST")
                        .help("Write a db list on screen (usefull to get the value of an opcode)")
                        .long("db")
                    )
                    .arg(Arg::new("LISTING_OUTPUT")
                        .help("Filename of the listing output.")
                        .long("lst")
                        .takes_value(true)
                    )
                    .arg(Arg::new("SYMBOLS_OUTPUT")
                        .help("Filename of the output symbols file.")
                        .long("sym")
                        .takes_value(true)
                    )
                    .group(
                        ArgGroup::new("ANY_OUTPUT")
                            .args(&["DB_LIST", "OUTPUT"])
                    )
					.arg(
						Arg::new("BASIC_HEADER")
							.help("Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive.")
							.long("basic")
							.alias("basicheader")
					)
					.arg(
						Arg::new("BINARY_HEADER")
							.help("Request a binary header")
							.long("binary")
							.alias("header")
							.alias("binaryheader")
                    )
                    .arg(
                        Arg::new("SNAPSHOT")
                            .help("Generate a snapshot")
                            .long("snapshot")
                            .alias("sna")
                    )
					.arg(
						Arg::new("CASE_INSENSITIVE")
							.help("Configure the assembler to be case insensitive.")
							.long("case-insensitive")
							.short('i') 
					)
                    .arg(
                        Arg::new("DOTTED_DIRECTIVES")
                            .help("Expect directives to by prefixed with a dot")
                            .long("directives-prefixed-by-dot")
                            .short('d')
                    )
                    .arg(
                        Arg::new("INCLUDE_DIRECTORIES")
                            .help("Provide additional directories used to search files.")
                            .long("include")
                            .short('I')
                            .takes_value(true)
                            .multiple_occurrences(true)
                            .number_of_values(1)
                    )
                    .arg(
                        Arg::new("DEFINE_SYMBOL")
                            .help("Provide a symbol with its value (default set to 1")
                            .long("define")
                            .short('D')
                            .takes_value(true)
                            .multiple_occurrences(true)
                            .number_of_values(1)
                    )
                    .arg(
                        Arg::new("LOAD_SYMBOLS")
                            .help("Load symbols from the given file")
                            .short('l')
                            .takes_value(true)
                            .multiple_occurrences(true)
                            .number_of_values(1)
                    )
                    .arg(
                        Arg::new("WERROR")
                        .help("Warning are considered to be errors")
                        .long("Werror")
                        .takes_value(false)
                    )
					.group( // only one type of header can be provided
						ArgGroup::new("HEADER")
							.args(&["BINARY_HEADER", "BASIC_HEADER"])
                    )
                    .group( // only one type of output can be provided
                        ArgGroup::new("ARTEFACT_TYPE")
                        .args(&["BINARY_HEADER", "BASIC_HEADER", "SNAPSHOT"])
                    )
}
