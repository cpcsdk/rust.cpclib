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
#![allow(clippy::cast_possible_truncation)]

use cpclib_asm::basm_utils::*;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );

    let matches = build_args_parser()
        .version(built_info::PKG_VERSION)
        .before_help(&desc_before[..])
        .get_matches();

    match process(&matches) {
        Ok((_env, warnings)) => {
            for warning in warnings {
                eprintln!("{}", warning);
            }

            std::process::exit(0);
        }
        Err(e) => {
            dbg!(&e);
            eprintln!("Error while assembling.\n{}", e);
            std::process::exit(-1);
        }
    }
}
