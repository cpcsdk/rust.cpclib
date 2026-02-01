//! Orgams ASCII format support for Amstrad CPC
//!
//! This crate provides utilities for reading and writing Orgams binary files,
//! a preprocessed Z80 assembly format used by the Orgams assembler.

#![feature(str_as_str)]
#![warn(missing_docs)]

pub mod binary_decoder;
pub mod convert;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

