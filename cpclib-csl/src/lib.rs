//! CSL (CPC Script Language) parser and types
//!
//! This crate provides parsing and manipulation of CSL scripts for controlling
//! Amstrad CPC emulators.

pub mod csl;
pub mod csl_parser;
pub mod error;

// Re-export commonly used types
pub use csl::*;
pub use csl_parser::{parse_csl_with_rich_errors, parse_instruction};
pub use error::CslError;
