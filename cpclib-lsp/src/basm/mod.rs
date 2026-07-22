//! LSP support for Z80 assembly in basm syntax (`.asm`/`.s`/`.z80` files).
//!
//! Each feature lives in its own file; they all extend the same
//! `AssemblyAnalyzer` type through separate `impl` blocks.
//!
//! Assembly sources can embed Locomotive BASIC blocks (`LOCOMOTIVE`
//! directive); those are detected here (`embedded_basic`) but analyzed by
//! the `locomotive` module — a one-directional `basm -> locomotive`
//! dependency.

use std::sync::Arc;

use cpclib_asm::parser::obtained::LocatedListing;
use dashmap::DashMap;
use tower_lsp::lsp_types::Url;

pub mod autocomplete;
pub mod call_hierarchy;
pub mod color;
pub mod command;
pub mod definition;
pub mod diagnostics;
pub mod disassemble;
pub mod embedded_basic;
pub mod expand;
pub mod format;
pub mod hover;
pub mod includes;
pub mod overflow;
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
///
/// Holds a parse-result cache keyed by document URI (see `parse.rs`'s
/// `parse_document`). `LocatedListing` is a self-referential type (built
/// with `ouroboros`) whose `ParserContext` field deliberately forbids
/// `Clone` - but that's a "don't accidentally deep-copy this" guard
/// (`ParserContext::clone()` panics because a derived clone would wrongly
/// duplicate a lazily-computed line/column lookup table instead of
/// resetting it, see `clone_with_state`), not a thread-safety one:
/// `LocatedListing` is verified `Send + Sync` (confirmed via a compile-time
/// `fn assert_send_sync<T: Send + Sync>()` probe), so caching it behind an
/// `Arc` - parsed once, shared by reference, never cloned - is exactly as
/// safe as it is for BASIC's fully-owned `LocatedBasicProgram`.
pub struct AssemblyAnalyzer {
    parse_cache: DashMap<Url, (i32, Result<Arc<LocatedListing>, Arc<LocatedListing>>)>
}

impl AssemblyAnalyzer {
    pub fn new() -> Self {
        Self {
            parse_cache: DashMap::new()
        }
    }

    /// Drop `uri`'s cached parse, if any - called on `textDocument/didClose`
    /// so a closed document's cache entry doesn't linger indefinitely.
    pub fn evict(&self, uri: &Url) {
        self.parse_cache.remove(uri);
    }
}

impl Default for AssemblyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
