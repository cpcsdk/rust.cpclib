//! Orgams ASCII format support for Amstrad CPC
//!
//! This crate provides utilities for reading and writing Orgams binary files,
//! a preprocessed Z80 assembly format used by the Orgams assembler.

#![warn(missing_docs)]

pub mod format;
pub mod decoder;

pub use format::{OrgamsFile, OrgamsHeader, LineMarker};
pub use decoder::{OrgamsDecoder, DecodedElement, Command, elements_to_z80_source};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
    
    #[test]
    fn test_magic_constant() {
        assert_eq!(format::MAGIC, b"ORGA");
    }
}
