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

use cpclib_imgconverter::{build_img2cpc_args_parser, built_info, process_img2cpc};

fn main() -> anyhow::Result<()> {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );

    let args = build_img2cpc_args_parser().before_help(desc_before);

    let matches = args.clone().get_matches();
    process_img2cpc(&matches, args)
}
