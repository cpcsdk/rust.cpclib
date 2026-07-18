//! LSP support for Z80 assembly in basm syntax (`.asm`/`.s`/`.z80` files).
//!
//! Each feature lives in its own file; they all extend the same
//! `AssemblyAnalyzer` type through separate `impl` blocks.
//!
//! Assembly sources can embed Locomotive BASIC blocks (`LOCOMOTIVE`
//! directive); those are detected here (`embedded_basic`) but analyzed by
//! the `locomotive` module — a one-directional `basm -> locomotive`
//! dependency.

pub mod autocomplete;
pub mod color;
pub mod command;
pub mod definition;
pub mod diagnostics;
pub mod embedded_basic;
pub mod format;
pub mod hover;
pub mod includes;
pub mod parse;
pub mod refactor;
pub mod semantic_tokens;
pub mod symbols;
pub mod timing;
pub mod token;

pub(crate) use token::semantic_tokens_legend;

/// Analyzer for Z80 assembly files using basm syntax.
///
/// Feature implementations are spread across this module's files, one
/// `impl AssemblyAnalyzer` block per concern.
pub struct AssemblyAnalyzer {}

impl AssemblyAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AssemblyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
