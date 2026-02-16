use clap::Parser;
use cpclib_catalog::{cli::CatalogApp, handle_catalog_command};
use simple_logger::SimpleLogger;

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), String> {
    let logger = SimpleLogger::new();
    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(logger)).unwrap();

    let args = CatalogApp::parse();
    handle_catalog_command(args)
}
