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

use std::io::{Read, Write};

use cpclib_basic::BasicProgram;
use cpclib_common::camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosHeader};
use cpclib_files::FileAndSupport;
use fs_err::File;

/// Locomotive BASIC manipulation tool
#[derive(Parser, Debug)]
#[command(name = "locomotive")]
#[command(about = "Locomotive BASIC manipulation tool", long_about = None)]
#[command(after_help = "Krusty/Benediction 2019")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Encode ASCII file to Amstrad BASIC binary format
    Encode {
        /// ASCII file containing the BASIC program
        #[arg(short, long, value_name = "FILE")]
        input: Utf8PathBuf,

        /// Output BASIC binary file
        #[arg(short, long, value_name = "FILE")]
        output: Utf8PathBuf,

        /// Add Amsdos header to the generated BASIC file
        #[arg(short = 'H', long)]
        header: bool,
    },
    /// Decode Amstrad BASIC binary to ASCII file
    Decode {
        /// BASIC binary file to decode
        #[arg(short, long, value_name = "FILE")]
        input: Utf8PathBuf,

        /// Output ASCII file (if not specified, prints to stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<Utf8PathBuf>,
    },
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode { input, output, header } => {
            encode_command(&input, &output, header)?;
        }
        Commands::Decode { input, output } => {
            decode_command(&input, output.as_ref())?;
        }
    }

    Ok(())
}

fn encode_command(input: &Utf8PathBuf, output: &Utf8PathBuf, with_header: bool) -> std::io::Result<()> {
    // Read the ASCII source file
    let basic_content: String = {
        let mut f = File::open(input)?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        content
    };

    // Parse the BASIC program
    let basic_tokens = BasicProgram::parse(basic_content)
        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unable to parse BASIC: {msg}")))?;

    // Get the bytes of the BASIC program
    let basic_bytes = basic_tokens.as_bytes();

    // Write to output file
    let mut f = File::create(output)?;

    // Add Amsdos header if requested
    if with_header {
        let header = AmsdosHeader::compute_basic_header(
            &AmsdosFileName::from_slice(output.as_str().as_bytes()),
            &basic_bytes
        );
        f.write_all(header.as_bytes().as_ref())?;
    }

    f.write_all(&basic_bytes)?;

    Ok(())
}

fn decode_command(input: &Utf8PathBuf, output: Option<&Utf8PathBuf>) -> std::io::Result<()> {
    // Read the BASIC binary file (with potential Amsdos header)
    let file = FileAndSupport::build(input);
    let content = file.content();

    // Decode the BASIC program
    let tokens = BasicProgram::decode(content.as_ref())
        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Error in the BASIC file: {msg}")))?;

    // Convert to ASCII representation
    let repr = tokens.to_string();

    // Write to output file or stdout
    if let Some(output_path) = output {
        let mut f = File::create(output_path)?;
        f.write_all(repr.as_bytes())?;
    } else {
        println!("{}", repr);
    }

    Ok(())
}
