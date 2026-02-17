use clap::Parser;
use cpclib_common::clap::{self, Command, CommandFactory};

include!(concat!(env!("OUT_DIR"), "/built.rs"));

#[derive(Parser)]
#[command(
    name = "borgams",
    about = "Benediction orgams manipulation",
    version = "0.11.0"
)]
struct BndOrgams {
    /// Input file (disc or tape image)
    #[arg(short, long, value_parser, help = "Input orgams file")]
    pub input: String,

    /// Output file (disc or tape image)
    #[arg(short, long, value_parser, help = "Output ascii file")]
    pub output: String,


    //pub cmd: Cat
}

#[derive(Parser)]
pub struct Cat {}


pub fn build_cli() -> Command {
    BndOrgams::command()
}