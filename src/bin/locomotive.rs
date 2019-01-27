
extern crate clap;
extern crate cpclib;

use cpclib::basic::BasicProgram;
use clap::*;


fn main() {
	let matches = App::new("locomotive")
						.about("Locomotive basic manipulation tool")
						.after_help("Krusty/Benediction 2019")
						.arg(
							Arg::with_name("BASIC_SOURCE")
								.long("basic")
								.short("b")
								.help("Source file that contains the basic program")
								.takes_value(true)
								.required(true)
						)
						.arg(

							Arg::with_name("HEADER")
								.long("header")
								.short("h")
								.help("Add the Amsdos header to the generated file")
						)
						.arg(
							Arg::with_name("OUTPUT")
								.help("Output file")
								.takes_value(true)
								.required(true)
						)
						.get_matches();

}