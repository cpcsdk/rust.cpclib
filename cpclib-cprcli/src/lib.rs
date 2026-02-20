use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::clap::{self, Arg, ArgAction};
use cpclib_common::utf8pathbuf_value_parser;

pub mod commands;

/// Build the clap Command for cprcli
pub fn build_command() -> clap::Command {
    clap::Command::new("cpclib-cprcli")
        .about("Command line CPR analysis")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("INFO")
                .help("Show information about the CPR")
                .long("info")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("SELECTED_BANKS")
                .help("Select banks of interest")
                .long("bank")
                .action(ArgAction::Append)
                .value_parser(0..32)
                .value_delimiter(',')
        )
        .arg(
            Arg::new("DUMP")
                .help("Get the memory of interest")
                .long("dump")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("INPUT")
                .help("The CPR file to read.")
                .long("cpr1")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(|p: &str| utf8pathbuf_value_parser(true)(p))
        )
        .arg(
            Arg::new("INPUT2")
                .help("The CPR file to compare with.")
                .long("cpr2")
                .required(false)
                .action(ArgAction::Set)
                .value_parser(|p: &str| utf8pathbuf_value_parser(true)(p))
        )
}
