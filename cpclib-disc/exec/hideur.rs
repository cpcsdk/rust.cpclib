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

use cpclib_disc::hideur::{hideur_build_arg_parser, hideur_handle, HideurError};

/// Convert a string to its unsigned 32 bits representation (to access to extra memory)
/// TODO share implementation
#[must_use]
pub fn string_to_nb(source: &str) -> u32 {
    let error = format!("Unable to read the value: {source}");
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    }
    else {
        source.parse::<u32>().expect(&error)
    }
}

fn main() -> Result<(), HideurError> {
    let matches = hideur_build_arg_parser().get_matches();
    hideur_handle(&matches)
}
