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

use cpclib_disc::{DskManagerError, dsk_manager_build_arg_parser, dsk_manager_handle};

// Still everything to do
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), DskManagerError> {
    let app = dsk_manager_build_arg_parser();
    let matches = app.get_matches();
    dsk_manager_handle(&matches)
}
