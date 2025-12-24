
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
use cpclib_disc::hideur::{HideurError, hideur_build_arg_parser, hideur_handle};

///
/// # Panics
///
/// Panics if the string cannot be parsed as a number in the expected format.
pub fn string_to_nb(source: &str) -> u32 {
    let error = format!("Unable to parse {source}");
    if let Some(stripped) = source.strip_prefix("0x") {
        u32::from_str_radix(stripped, 16).expect(&error)
    } else {
        source.parse().expect(&error)
    }
}

fn main() -> Result<(), HideurError> {
    let matches = hideur_build_arg_parser().get_matches();
    hideur_handle(&matches)
}
