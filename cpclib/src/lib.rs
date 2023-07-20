// TryFrom is used in the assembler stuff
#![feature(type_ascription)]
#![feature(associated_type_bounds)]
#![feature(proc_macro_hygiene)]
// Notes for later when clippy will work:
// https://rust-lang.github.io/rust-clippy/master/index.html#identity_op must be deactivated
#![deny(
    missing_docs,
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
#![allow(
    clippy::module_name_repetitions,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::module_inception,
    clippy::identity_op
)]

//! cpclib aims at providing tools that help cross-development for the Amstrad CPC.
//! It is mainly focused on the creation of demos for the Amstrad CPC but could be used for games or tools.
//! Warning: none of the proposed features is fully fonctional or complete ! But hey should be correct.
//! I progressively implement them depending on my needs for my current demo.
//!
//! So what can be found there:
//!  - z80 assembler (remember, it only assembles mnemonics I needed for my project)
//! - image conversion
//! - dsk manipulation
//!
//! Why releasing it publicly whereas no-one else will use it ? ;) Just because it is simpler to manage public crates with cargo. There is nothing secret here, and I welcome any contribution.

#![recursion_limit = "512"]

pub use cpclib_image::ga::{Ink, Palette, Pen};
/// CPC Wifi extension related stuff. Useable
#[cfg(any(feature = "xferlib", feature = "xfer"))]
pub use cpclib_xfer as xfer;
pub use {
    cpclib_asm as asm, cpclib_basic as basic, cpclib_common as common,
    cpclib_crunchers as crunchers, cpclib_disc as disc, cpclib_image as image, cpclib_sna as sna,
    cpclib_z80emu as z80emu
};

/// Disk (edsk) manipulation. WIP
// Some reexports
pub use crate::disc::edsk::ExtendedDsk;
