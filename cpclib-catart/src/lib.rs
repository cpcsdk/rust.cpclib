//! Catart-related functionality for Amstrad CPC demos
//! 
//! This crate provides utilities for working with Catart demo code,
//! building on top of the BASIC parsing capabilities from cpclib-basic.

#![warn(missing_docs)]

pub mod basic_chars;
/// Conversion utilities
pub mod convert;
/// Error definitions
pub mod error;
/// Command definitions (e.g. PRINT, LOCATE, etc.)
pub mod basic_command;
pub mod char_command;
pub mod interpret;

pub mod built_info {
    //! Build-time information
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        // Basic test to ensure the crate compiles
        assert!(true);
    }
}
