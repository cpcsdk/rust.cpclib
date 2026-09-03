//! Orgams ASCII format support for Amstrad CPC
//!
//! This crate provides utilities for reading and writing Orgams binary files,
//! a preprocessed Z80 assembly format used by the Orgams assembler.

#![warn(missing_docs)]

/// Winnow-based parser and encoder for the Orgams binary format.
pub mod binary_decoder;
/// High-level conversion helpers built on top of [`binary_decoder`].
pub mod convert;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
