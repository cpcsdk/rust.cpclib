//! LSP support for Locomotive BASIC (`.bas` files), also reused by the basm
//! module for BASIC blocks embedded in Z80 assembly (`LOCOMOTIVE` directive).
//!
//! Each feature lives in its own file; they all extend the same
//! `BasicAnalyzer` type through separate `impl` blocks.

pub mod autocomplete;
pub mod command;
pub mod definition;
pub mod diagnostics;
pub mod hover;
pub mod semantic_tokens;
pub mod symbols;
pub mod token;

/// Analyzer for Locomotive BASIC documents.
///
/// Feature implementations are spread across this module's files, one
/// `impl BasicAnalyzer` block per concern.
pub struct BasicAnalyzer;

impl BasicAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
