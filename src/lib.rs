// TryFrom is used in the assembler stuff
#![feature(try_from)]
#![feature(type_alias_enum_variants)]
#![feature(type_ascription)]
#![feature(custom_attribute)]


// Notes for later when clippy will work:
// https://rust-lang.github.io/rust-clippy/master/index.html#identity_op must be deactivated

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

#![recursion_limit="512"]

#[macro_use]
extern crate bitfield;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate nom;

#[cfg(feature = "xferlib")]
extern crate curl;
#[cfg(feature = "xferlib")]
extern crate reqwest;

#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate serde_derive;


#[macro_use]
extern crate dbg;
#[macro_use]
extern crate failure;

#[macro_use]
extern crate getset;

/// Screen emulation. Unknown state ;)
pub mod screen;

/// CPC pixels conversion routines. Useable
pub mod pixels;

/// CPC Image manipulation. Useable.
pub mod image;

/// PC to CPC image conversions. WIP
pub mod imageconverter;

/// Gate Array specific objects. Finished.
pub mod ga;

pub mod asm;

/// Z80 tokens manipulations. Useable
pub mod assembler;

// Basic program manipulation. WIP
pub mod basic;

/// Z80 emulation. WIP
pub mod z80emu;

/// Snapshot manipulation. Useable
pub mod sna;

/// Disk (edsk) manipulation. WIP
pub mod disc;

#[cfg(any(feature = "xferlib", feature = "xfer"))]
pub mod xfer;

// Some reexports
pub use crate::disc::edsk::ExtendedDsk;
pub use crate::ga::{Ink, Palette, Pen};

pub mod util {
    /**
     * Convert a string to its unsigned 32 bits representation (to access to extra memory)
     */
    pub fn string_to_nb(source: &str) -> u32 {
        let error = format!("Unable to read the value: {}", source);
        if source.starts_with("0x") {
            u32::from_str_radix(&source[2..], 16).expect(&error)
        } else {
            source.parse::<u32>().expect(&error)
        }
    }

}
