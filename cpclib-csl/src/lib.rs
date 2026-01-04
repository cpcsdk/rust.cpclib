//! CSL (CPC Script Language) parser and types
//!
//! This crate provides parsing and manipulation of CSL scripts for controlling
//! Amstrad CPC emulators.

pub mod csl;
pub mod csl_parser;

// Re-export commonly used types
pub use csl::*;
pub use csl_parser::{parse_csl, parse_instruction};
