use std::{fs, process};

use cpclib_common::clap::{Arg, ArgAction, Command};
use cpclib_csl::parse_csl_with_rich_errors;

fn main() {
    let matches = Command::new("cpclib-cslcli")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Krusty/Benediction")
        .about("CSL (CPC Script Language) parser and validator")
        .arg(
            Arg::new("file")
                .help("Path to the CSL file to parse")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(ArgAction::SetTrue)
        )
        .get_matches();

    let file_path = matches.get_one::<String>("file").unwrap();
    let verbose = matches.get_flag("verbose");

    // Read the file
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    // Parse the CSL file
    match parse_csl_with_rich_errors(&content, Some(file_path.to_string())) {
        Ok(script) => {
            if verbose {
                eprintln!("Successfully parsed {} instructions", script.len());
            }
            // Output the parsed script to stdout
            print!("{}", script);
            process::exit(0);
        },
        Err(e) => {
            // Output error to stderr
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
