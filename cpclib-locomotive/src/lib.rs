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

pub use clap::{CommandFactory, Parser, Subcommand};
use cpclib_basic::BasicProgram;
use cpclib_common::camino::Utf8PathBuf;
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosHeader};
use cpclib_files::FileAndSupport;
use fs_err::File;

/// Locomotive BASIC manipulation tool
#[derive(Parser, Debug)]
#[command(name = "locomotive")]
#[command(about = "Locomotive BASIC manipulation tool", long_about = None)]
#[command(after_help = "Krusty/Benediction 2019-2026")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
        header: bool
    },
    /// Decode Amstrad BASIC binary to ASCII file
    Decode {
        /// BASIC binary file to decode
        #[arg(short, long, value_name = "FILE")]
        input: Utf8PathBuf,

        /// Output ASCII file (if not specified, prints to stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<Utf8PathBuf>
    }
}

pub fn handle_locomotive_arguments(cli: Cli) -> std::io::Result<()> {
    match cli.command {
        Commands::Encode {
            input,
            output,
            header
        } => {
            encode_command(&input, &output, header)?;
        },
        Commands::Decode { input, output } => {
            decode_command(&input, output.as_ref())?;
        }
    }

    Ok(())
}

fn encode_command(
    input: &Utf8PathBuf,
    output: &Utf8PathBuf,
    with_header: bool
) -> std::io::Result<()> {
    // Read the ASCII source file
    // TODO aad the ability to read files from supports
    let basic_content: String = {
        let mut f = File::open(input)?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        content
    };

    // Parse the BASIC program
    let basic_tokens = BasicProgram::parse(basic_content).map_err(|msg| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unable to parse BASIC: {msg}")
        )
    })?;

    // Get the bytes of the BASIC program
    let basic_bytes = basic_tokens.as_bytes();

    // Write to output file
    let mut f = File::create(output)?;

    // Add Amsdos header if requested
    if with_header {
        // `AmsdosFileName::from_slice` expects a raw 12-byte on-disk catalog
        // entry buffer (`[user, name x8, extension x3]`), not a human
        // readable path - feeding it `output`'s raw path bytes directly
        // panics for any filename under 12 bytes and silently produces a
        // garbled name/extension otherwise (the leading directory
        // components and punctuation end up misread as the user byte and
        // name/extension characters). Only the file's own basename belongs
        // in the header, parsed through the string-aware
        // `TryFrom<&str>` (`user:name.extension` syntax, defaulting to
        // user 0) instead.
        let basename = output.file_name().unwrap_or_else(|| output.as_str());
        let amsdos_name = AmsdosFileName::try_from(basename).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid Amsdos filename {basename:?}: {e}")
            )
        })?;
        let header = AmsdosHeader::compute_basic_header(&amsdos_name, &basic_bytes);
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
    let tokens = BasicProgram::decode(content.as_ref()).map_err(|msg| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Error in the BASIC file: {msg}")
        )
    })?;

    // Convert to ASCII representation
    let repr = tokens.to_string();

    // Write to output file or stdout
    if let Some(output_path) = output {
        let mut f = File::create(output_path)?;
        f.write_all(repr.as_bytes())?;
    }
    else {
        println!("{repr}");
    }

    Ok(())
}

/// Build the clap Command for testing purposes
#[must_use]
pub fn build_command() -> clap::Command {
    Cli::command()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real regression repro: `--header` used to build the on-disk
    /// `AmsdosFileName` from the raw *path* bytes via `AmsdosFileName::
    /// from_slice` (meant for a 12-byte on-disk catalog entry buffer, not a
    /// human-readable path) - a short filename like `CATART.BAS` (10 bytes)
    /// panicked outright (`array_ref!` slicing 12 bytes out of a 10-byte
    /// slice), and a longer path silently produced a garbled name/extension
    /// from whatever the path's first 12 raw bytes happened to be (found via
    /// a real project's `albi/CATART.BAS`, whose header decoded to user
    /// byte `'a'` (0x61) and name/extension `"lbi/CATA"`/`"RT."` instead of
    /// user 0 / `"CATART  "` / `"BAS"`).
    #[test]
    fn header_uses_the_output_files_own_basename_not_raw_path_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "cpclib_locomotive_test_{}_{}",
            std::process::id(),
            "header_basename"
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let input = Utf8PathBuf::from_path_buf(dir.join("in.asc")).unwrap();
        std::fs::write(&input, "10 PRINT \"HI\"\n").unwrap();

        // A short output filename (under 12 bytes) is exactly what used to
        // panic outright.
        let output = Utf8PathBuf::from_path_buf(dir.join("CATART.BAS")).unwrap();

        encode_command(&input, &output, true).expect("must not panic or error");

        let written = std::fs::read(&output).unwrap();
        assert!(written.len() >= 128, "expected at least a full Amsdos header");
        assert_eq!(&written[0], &0u8, "user byte must be 0, not derived from path bytes");
        assert_eq!(
            &written[1..9],
            b"CATART  ",
            "name field must be the basename ('CATART'), space-padded to 8 bytes"
        );
        assert_eq!(
            &written[9..12],
            b"BAS",
            "extension field must be 'BAS', not raw path bytes"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Same bug, but via a *long* output path (a real directory prefix) -
    /// this used to not panic (12+ raw path bytes available to slice) but
    /// silently produce a garbled header instead, which is arguably worse
    /// (no crash to notice, just a wrong on-disk file).
    #[test]
    fn header_basename_extraction_ignores_directory_components() {
        let dir = std::env::temp_dir().join(format!(
            "cpclib_locomotive_test_{}_{}",
            std::process::id(),
            "header_dir_prefix"
        ));
        std::fs::create_dir_all(dir.join("albi")).unwrap();

        let input = Utf8PathBuf::from_path_buf(dir.join("in.asc")).unwrap();
        std::fs::write(&input, "10 PRINT \"HI\"\n").unwrap();

        let output = Utf8PathBuf::from_path_buf(dir.join("albi/CATART.BAS")).unwrap();

        encode_command(&input, &output, true).expect("must not panic or error");

        let written = std::fs::read(&output).unwrap();
        assert_eq!(&written[0], &0u8);
        assert_eq!(&written[1..9], b"CATART  ");
        assert_eq!(&written[9..12], b"BAS");

        std::fs::remove_dir_all(&dir).ok();
    }
}
