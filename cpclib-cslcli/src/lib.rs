use cpclib_common::clap::{Arg, ArgAction, Command};

/// Build the clap Command for cslcli  
pub fn build_command() -> Command {
    Command::new("cpclib-cslcli")
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
}
