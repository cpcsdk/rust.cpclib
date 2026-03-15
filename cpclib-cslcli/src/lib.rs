use anyhow::{Context, Result};
use clap::Parser;
use cpclib_common::event::EventObserver;
use cpclib_csl::parse_csl_with_rich_errors;

/// CSL (CPC Script Language) parser and validator
#[derive(Parser, Debug)]
#[command(name = "csl")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Krusty/Benediction")]
#[command(about = "CSL (CPC Script Language) parser and validator")]
pub struct CslCliArgs {
    /// Path to the CSL file to parse
    #[arg(required = true, index = 1)]
    pub file: String,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool
}

/// Run the CSL CLI with the provided arguments
pub fn run<O: EventObserver>(args: &CslCliArgs, o: &O) -> Result<()> {
    // Read the file
    let content = fs_err::read_to_string(&args.file)
        .context(format!("Failed to read file '{}'", args.file))?;

    // Parse the CSL file
    let script = parse_csl_with_rich_errors(&content, Some(args.file.clone()))
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if args.verbose {
        o.emit_stderr(&format!("Successfully parsed {} instructions", script.len()));
    }

    // Output the parsed script to stdout
    o.emit_stdout(&script.to_string());

    Ok(())
}
