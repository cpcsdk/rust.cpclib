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

use std::fs::File;
use std::io::{Read, Write};

use cpclib_catalog::cli::CatalogCommand;
/// Catalog tool manipulator.
use cpclib_catalog::{catalog_extraction, catalog_to_basic_listing, catalog_to_catart_commands};
use cpclib_catart::basic_command::BasicCommandList;
use cpclib_catart::char_command::CharCommandList;
use cpclib_catart::entry::{Catalog, CatalogType, PrintableEntryFileName, ScreenMode, SerialCatalogBuilder, UnifiedCatalog};
use cpclib_catart::interpret::{Interpreter, Locale, Mode, screens_are_equal, display_screen_diff};
use cpclib_basic::BasicProgram;
use cpclib_common::clap::value_parser;
use cpclib_common::num::Num;
use cpclib_disc::amsdos::{AmsdosEntries, AmsdosManagerNonMut, BlocIdx};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
use cpclib_disc::{open_disc, AnyDisc};
use log::{error, info};
use simple_logger::SimpleLogger;
use cli::CatalogCommand;


#[allow(clippy::too_many_lines)]
fn main() -> Result<(), String> {
    let logger = SimpleLogger::new();
    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(logger)).unwrap();

    let args = CatalogApp::parse();
    handle_catalog_command(args)
}
