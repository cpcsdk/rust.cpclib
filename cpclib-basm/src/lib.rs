#![feature(box_patterns)]

use std::fmt::Display;
use std::fs::{self, File};
use std::io;
use std::io::Write;
use std::path::Path;

use cpclib_asm::assembler::file::{get_filename, handle_source_encoding};
use cpclib_asm::preamble::*;
use cpclib_asm::progress::{normalize, Progress};
use cpclib_common::clap::builder::{PossibleValue, PossibleValuesParser};
use cpclib_common::clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command, ValueHint};
use cpclib_common::itertools::Itertools;
use cpclib_common::{clap, lazy_static};
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosManager};
#[cfg(feature = "xferlib")]
use cpclib_xfer::CpcXfer;

use crate::embedded::EmbeddedFiles;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug)]
pub enum BasmError {
    //#[fail(display = "IO error: {}", io)]
    Io {
        io: io::Error,
        ctx: String
    },

    // #[fail(display = "Assembling error: {}", error)]
    AssemblerError {
        error: AssemblerError
    },
    ErrorWithListing {
        error: Box<BasmError>,
        listing: LocatedListing
    },

    // #[fail(display = "Invalid Amsdos filename: {}", filename)]
    InvalidAmsdosFilename {
        filename: String
    },

    // #[fail(display = "{} is not a valid directory.", path)]
    NotAValidDirectory {
        path: String
    },

    //  #[fail(display = "{} is not a valid file.", file)]
    NotAValidFile {
        file: String
    },

    ListingGeneration {
        msg: String
    },

    InvalidSymbolFile {
        msg: String
    },

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

            BasmError::InvalidSymbolFile { msg } => {
                write!(f, "Error when reading the symbol table: {}", msg)
            }

            BasmError::InvalidArgument(msg) => {
                write!(f, "Invalid argument: {}", msg)
            }
            BasmError::ErrorWithListing {
                box error,
                listing: _
            } => error.fmt(f)
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
) -> Result<(LocatedListing, ParserOptions), BasmError> {
    let inline_fname = "<inline code>";
    let filename = matches
        .get_one::<String>("INPUT")
        .map(AsRef::as_ref)
        .unwrap_or(inline_fname);

    let show_progress = matches.get_flag("PROGRESS");

    // prepare the context for the included directories
    let mut options = ParserOptions::default();
    options.set_dotted_directives(matches.get_flag("DOTTED_DIRECTIVES"));
    options.show_progress = show_progress;

    match std::env::current_dir() {
        Ok(cwd) => {
            options.add_search_path(cwd)?;
        }
        Err(_) => todo!()
    }
    options.add_search_path_from_file(&filename); // we ignore the potential error
    if let Some(directories) = matches.get_many::<String>("INCLUDE_DIRECTORIES") {
        for directory in directories {
            if !Path::new(directory).is_dir() {
                return Err(BasmError::NotAValidDirectory {
                    path: directory.to_owned()
                });
            }
            options.add_search_path(directory)?;
        }
    }

    let mut builder = options.clone().context_builder();

    // get the source code if any
    let code = if matches.contains_id("INPUT") {
        builder = builder.set_current_filename(&filename);
        let filename = get_filename(filename, &options, None)?;
        let content = fs::read(filename.clone()).map_err(|e| {
            AssemblerError::IOError {
                msg: format!("Unable to open {:?}. {}", filename, e)
            }
        })?;
        handle_source_encoding(filename.to_str().unwrap(), &content)?
    }
    else if let Some(code) = matches.get_one::<String>("INLINE") {
        builder = builder.set_context_name("INLINED CODE");
        format!(" {}", code)
    }
    else {
        panic!("No code provided to assemble");
    };

    let fname = builder
        .current_filename()
        .map(|fname| normalize(fname))
        .unwrap_or_else(|| builder.context_name().unwrap())
        .to_owned();

    if options.show_progress {
        Progress::progress().add_parse(&fname);
    };

    let res = crate::parse_z80_with_context_builder(code, builder)
        .map_err(|e| BasmError::from(AssemblerError::AlreadyRenderedError(e.to_string())));

    if options.show_progress {
        Progress::progress().remove_parse(&fname);
    };

    Ok((res?, options))
}

/// Assemble the given code
/// TODO use options to configure the base symbole table
pub fn assemble<'arg>(
    matches: &'arg ArgMatches,
    listing: &LocatedListing,
    parse_options: ParserOptions
) -> Result<Env, BasmError> {
    let _show_progress = matches.get_flag("PROGRESS");

    let mut assemble_options = AssemblingOptions::default();
    assemble_options.set_case_sensitive(!matches.get_flag("CASE_INSENSITIVE"));

    // TODO add symbols if any
    if let Some(files) = matches.get_many::<String>("LOAD_SYMBOLS") {
        for path in files {
            let file = Path::new(path);
            if !file.is_file() {
                return Err(BasmError::NotAValidFile {
                    file: path.to_owned()
                });
            }

            let content = file::read_source(file, &parse_options)?;
            let builder = ParserContextBuilder::default().set_state(ParsingState::SymbolsLimited);
            let listing = parse_z80_with_context_builder(&content, builder)?;
            for token in listing.iter() {
                if token.is_equ() {
                    let symbol = token.equ_symbol();
                    let value = token
                        .equ_value()
                        .eval()
                        .map_err(|e| {
                            let _span = token.possible_span().unwrap();
                            let span = token.possible_span().unwrap();
                            let e: AssemblerError = e.into();
                            e.locate(span.clone())
                        })
                        .map_err(|e| BasmError::InvalidSymbolFile { msg: e.to_string() })?;

                    assemble_options
                        .symbols_mut()
                        .assign_symbol_to_value(symbol, value)
                        .map_err(|e| {
                            let span = token.possible_span().unwrap();
                            let e: AssemblerError = e.into();
                            e.locate(span.clone())
                        })
                        .map_err(|e| BasmError::InvalidSymbolFile { msg: e.to_string() })?;
                }
            }
        }
    }

    // Get the variables definition
    if let Some(definitions) = matches.get_many::<String>("DEFINE_SYMBOL") {
        for definition in definitions {
            let (symbol, value) = {
                match definition.split_once("=") {
                    Some((symbol, value)) => (symbol, value),
                    None => (definition.as_str(), "1")
                }
            };

            let ctx = ParserOptions::default()
                .context_builder()
                .set_context_name("BASM OPTIONS")
                .build(value);

            let span = Z80Span::new_extra(value, &ctx);
            let value = /*cpclib_common::*/parse_value(span)
                    .map_err(|_e| BasmError::InvalidArgument(definition.to_string()))
                    ?
                    .1;

            assemble_options
                .symbols_mut()
                .assign_symbol_to_value(symbol, value.eval()?)
                .map_err(|_e| BasmError::InvalidArgument(definition.to_string()))?;
        }
    }

    if let Some(dest) = matches.get_one::<String>("LISTING_OUTPUT") {
        if dest == "-" {
            assemble_options.write_listing_output(std::io::stdout());
        }
        else {
            let file = File::create(dest).map_err(|e| {
                BasmError::Io {
                    io: e,
                    ctx: format!("creating {}", dest)
                }
            })?;
            assemble_options.write_listing_output(file);
        }
    }

    let options = EnvOptions::new(parse_options, assemble_options);
    let (_tokens, mut env) =
        visit_tokens_all_passes_with_options(&listing, options).map_err(|(_t_, mut env, e)| {
            env.handle_print(); // do the prints even if there is an assembling issue
            BasmError::AssemblerError {
                error: AssemblerError::AlreadyRenderedError(e.to_string())
            }
        })?;

    env.handle_post_actions().map_err(|e| {
        BasmError::AssemblerError {
            error: AssemblerError::AlreadyRenderedError(e.to_string())
        }
    })?;

    if let Some(dest) = matches.get_one::<String>("SYMBOLS_OUTPUT") {
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
    let show_progress = matches.get_flag("PROGRESS");

    if matches.get_flag("SNAPSHOT")
        && !matches.contains_id("TO_M4")
        && !matches.contains_id("OUTPUT")
    {
        return Err(BasmError::InvalidArgument(
            "You have not provided an output file name".to_owned()
        ));
    }

    if matches.get_flag("SNAPSHOT") && matches.contains_id("OUTPUT") {
        // Get the appropriate filename
        let pc_filename = matches.get_one::<String>("OUTPUT").unwrap();

        env.save_sna(pc_filename.clone()).map_err(|e| {
            BasmError::Io {
                io: e,
                ctx: format!("saving \"{}\"", pc_filename)
            }
        })?;

        #[cfg(feature = "xferlib")]
        match matches.get_one::<String>("TO_M4") {
            Some(m4) => {
                #[cfg(feature = "indicatif")]
                let bar = if show_progress {
                    Some(Progress::progress().add_bar("Send to M4"))
                }
                else {
                    None
                };

                let xfer = CpcXfer::new(m4);
                xfer.upload_and_run(pc_filename, None)
                    .expect("An error occured while transfering the snapshot");

                #[cfg(feature = "indicatif")]
                if let Some(bar) = bar {
                    Progress::progress().remove_bar_ok(&bar);
                }
            }
            None => {}
        }
    }
    else if cfg!(feature = "xferlib")
        && matches.contains_id("TO_M4")
        && !matches.contains_id("OUTPUT")
    {
        #[cfg(feature = "xferlib")]
        {
            let sna = env.sna();
            let m4 = matches.get_one::<String>("TO_M4").unwrap();

            #[cfg(feature = "indicatif")]
            let bar = if show_progress {
                Some(Progress::progress().add_bar("Send to M4"))
            }
            else {
                None
            };

            let xfer = CpcXfer::new(m4);
            xfer.upload_and_run_sna(sna)
                .expect("An error occured while transfering the snapshot");

            #[cfg(feature = "indicatif")]
            if let Some(bar) = bar {
                Progress::progress().remove_bar_ok(&bar);
            }
        }
    }
    else if matches.contains_id("OUTPUT") || matches.get_flag("DB_LIST") {
        // Collect the produced bytes
        let binary = env.produced_bytes();

        if matches.get_flag("DB_LIST") {
            let bytes = env.produced_bytes();
            if !bytes.is_empty() {
                let listing = Listing::from(bytes.as_ref());
                println!("{}", PrintableListing::from(&Listing::from(listing)));
            }
        }
        else {
            debug_assert!(matches.contains_id("OUTPUT"));
            let pc_filename = matches.get_one::<String>("OUTPUT").unwrap();
            if pc_filename.to_lowercase().ends_with(".sna") && !matches.get_flag("SNAPSHOT") {
                eprintln!(
                    "[WARNING] You are saving a file with .sna extension without using --sna flag"
                );
            }
            let amsdos_filename = AmsdosFileName::try_from(pc_filename.as_ref());

            // Raise an error if the filename is not compatible with the header
            if (matches.get_flag("BINARY_HEADER") || matches.get_flag("BASIC_HEADER"))
                && amsdos_filename.is_err()
            {
                return Err(BasmError::InvalidAmsdosFilename {
                    filename: pc_filename.to_string()
                });
            }

            // Compute the headers if needed
            let header = if matches.get_flag("BINARY_HEADER") {
                AmsdosManager::compute_binary_header(
                    &amsdos_filename.unwrap(),
                    env.loading_address().unwrap(),
                    env.execution_address().unwrap(),
                    &binary
                )
                .as_bytes()
                .to_vec()
            }
            else if matches.get_flag("BASIC_HEADER") {
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
    // Handle the display of embedded files list
    if matches.get_flag("LIST_EMBEDDED") {
        use crate::embedded::EmbeddedFiles;
        for fname in EmbeddedFiles::iter() {
            println!("{}", fname)
        }
        std::process::exit(0);
    }

    // Handle the display of a specific embedded file
    if let Some(fname) = matches.get_one::<String>("VIEW_EMBEDDED") {
        use crate::embedded::EmbeddedFiles;

        match EmbeddedFiles::get(fname) {
            Some(content) => {
                println!("{}", std::str::from_utf8(content.data.as_ref()).unwrap());
                std::process::exit(0);
            }
            None => {
                eprintln!("Embedded file {fname} does not exist");
                std::process::exit(-1);
            }
        }
    }

    // standard assembling
    let (listing, options) = parse(matches)?;
    let env = assemble(matches, &listing, options).map_err(move |error| {
        BasmError::ErrorWithListing {
            error: Box::new(error),
            listing
        }
    })?;

    //  eprintln!("TODO: include parse warnings");
    // warnings.extend_from_slice(env.warnings());
    let warnings = env.warnings().to_vec();

    if matches.get_flag("WERROR") && !warnings.is_empty() {
        const KEPT: usize = 10;

        if warnings.len() > KEPT {
            eprintln!("Warnings are considered to be errors. The first 10 have been kept.");
        }
        else {
            eprintln!("Warnings are considered to be errors.");
        }

        // keep only the first 10
        return Err(AssemblerError::MultipleErrors {
            errors: warnings.into_iter().take(KEPT).collect_vec()
        }
        .into());
    }
    else {
        save(matches, &env)?;
        return Ok((env, warnings));
    }
}

lazy_static::lazy_static! {
    static ref EMBEDDED_FILES_NAME: Vec<String> = EmbeddedFiles::iter().map(|s|s.into_owned()).collect_vec();
    static ref EMBEDDED_FILES: Vec<PossibleValue>  = EMBEDDED_FILES_NAME.iter()
        .map(|s| PossibleValue::from(s.as_str()))
        .collect_vec();

}

/// Generated the clap Commands
pub fn build_args_parser() -> clap::Command {
    let cmd = Command::new("basm")
					.author("Krusty/Benediction")
                    .version(built_info::PKG_VERSION)
					.about("Benediction ASM -- z80 assembler that mainly targets Amstrad CPC")
                    .after_help("Still a Work In Progress assembler")
                    .arg(
                        Arg::new("INLINE")
                            .help("Z80 code is provided inline")
                            .long("inline")
                    )
                    .arg(
						Arg::new("INPUT")
							.help("Input file to read.")
                            .value_hint(ValueHint::FilePath)
                    )

					.arg(
						Arg::new("OUTPUT")
							.help("Filename of the output.")
							.short('o')
							.long("output")
                            .value_hint(ValueHint::FilePath)
					)
					.arg(
                        Arg::new("DB_LIST")
                        .help("Write a db list on screen (usefull to get the value of an opcode)")
                        .long("db")
                        .action(ArgAction::SetTrue)
                    )
                    .arg(Arg::new("LISTING_OUTPUT")
                        .help("Filename of the listing output.")
                        .long("lst")
                        .value_hint(ValueHint::FilePath)
                    )
                    .arg(Arg::new("SYMBOLS_OUTPUT")
                        .help("Filename of the output symbols file.")
                        .long("sym")                      
                        .value_hint(ValueHint::FilePath)
                    )
                    .group(
                        ArgGroup::new("ANY_OUTPUT")
                            .args(&["DB_LIST", "OUTPUT"])
                            .required(false)
                    )
					.arg(
						Arg::new("BASIC_HEADER")
							.help("Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive).")
							.long("basic")
							.alias("basicheader")
                            .action(ArgAction::SetTrue)
					)
					.arg(
						Arg::new("BINARY_HEADER")
							.help("Request a binary header")
							.long("binary")
							.alias("header")
							.alias("binaryheader")
                            .action(ArgAction::SetTrue)
                    )
                    .arg(
                        Arg::new("SNAPSHOT")
                            .help("Generate a snapshot")
                            .long("snapshot")
                            .alias("sna")
                            .action(ArgAction::SetTrue)
                    )
					.arg(
						Arg::new("CASE_INSENSITIVE")
							.help("Configure the assembler to be case insensitive.")
							.long("case-insensitive")
							.short('i')
                            .action(ArgAction::SetTrue)
					)
                    .arg(
                        Arg::new("DOTTED_DIRECTIVES")
                            .help("Expect directives to by prefixed with a dot")
                            .long("directives-prefixed-by-dot")
                            .short('d')
                            .action(ArgAction::SetTrue)
                    )
                    .arg(
                        Arg::new("INCLUDE_DIRECTORIES")
                            .help("Provide additional directories used to search files")
                            .long("include")
                            .short('I')
                            .action(ArgAction::Append)
                            .number_of_values(1)
                            .value_hint(ValueHint::DirPath)
                    )
                    .arg(
                        Arg::new("DEFINE_SYMBOL")
                            .help("Provide a symbol with its value (default set to 1)")
                            .long("define")
                            .short('D')
                            .action(ArgAction::Append)
                            .number_of_values(1)
                    );

    let cmd = if cfg!(feature = "xferlib") {
        cmd.arg(
            Arg::new("TO_M4")
                .help("Provide the IP address of the M4")
                .long("m4")
        )
    }
    else {
        cmd
    };

    cmd.arg(
        Arg::new("LOAD_SYMBOLS")
            .help("Load symbols from the given file")
            .short('l')
            .action(ArgAction::Append)
            .number_of_values(1)
    )
    .arg(
        Arg::new("WERROR")
            .help("Warning are considered to be errors")
            .long("Werror")
            .action(ArgAction::SetTrue)
    )
    .arg(
        Arg::new("PROGRESS")
            .help("Show a progress bar.")
            .long("progress")
            .action(ArgAction::SetTrue)
    )
    .arg(
        Arg::new("LIST_EMBEDDED")
            .help("List the embedded files")
            .long("list-embedded")
            .action(ArgAction::SetTrue)
    )
    .arg(
        Arg::new("VIEW_EMBEDDED")
            .help("Display one specific embedded file")
            .long("view-embedded")
            .number_of_values(1)
            .value_parser(PossibleValuesParser::new(EMBEDDED_FILES.iter().cloned()))
    )
    .group(
        // only one type of header can be provided
        ArgGroup::new("HEADER").args(&["BINARY_HEADER", "BASIC_HEADER"])
    )
    .group(
        // only one type of output can be provided
        ArgGroup::new("ARTEFACT_TYPE").args(&["BINARY_HEADER", "BASIC_HEADER", "SNAPSHOT"])
    )
    .group(
        ArgGroup::new("ANY_INPUT")
            .args(&["INLINE", "INPUT", "LIST_EMBEDDED", "VIEW_EMBEDDED"])
            //  .required(true)
            .conflicts_with("version")
    )
}
