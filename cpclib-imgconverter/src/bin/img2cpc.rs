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
#![allow(unused)]

use anyhow;
use cpclib_imgconverter::{build_args_parser, process};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() -> anyhow::Result<()> {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );


    let args = build_args_parser()
        .before_help(desc_before);



    let matches = args.clone().get_matches();
    process(&matches, args);

    Ok(())
}
