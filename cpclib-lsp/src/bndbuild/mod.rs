//! LSP support for bndbuild build files (`bndbuild.yml` / `bnd.build` /
//! `build.bnd`): YAML rules + Jinja templating + task command lines.
//!
//! Each feature lives in its own file; they all extend the same
//! `BuildFileAnalyzer` type through separate `impl` blocks.

use std::sync::Arc;

use dashmap::DashMap;
use tower_lsp::lsp_types::Url;

pub mod autocomplete;
pub mod call_hierarchy;
pub mod command;
pub mod definition;
pub mod delegated_help;
pub mod diagnostics;
pub mod hover;
pub mod internal_commands;
pub mod jinja;
pub mod macro_expand;
pub mod semantic_tokens;
pub mod sourcemap;
pub mod symbols;
pub mod token;

/// Analyzer for build files (YAML with Jinja templates).
///
/// Feature implementations are spread across this module's files, one
/// `impl BuildFileAnalyzer` block per concern.
///
/// Holds a cache of `sourcemap::expand_with_source_map`'s result (a full
/// Jinja parse+render of the whole document, including filesystem reads for
/// every `{% include %}`) keyed by document URI, mirroring `basm`/
/// `locomotive`'s `parse_cache` — without it, every hover/definition/
/// symbols/diagnostics/call-hierarchy/semantic-tokens request that needs
/// the expanded text redid this from scratch, even multiple times within
/// the same request (see `diagnostics::analyze`).
pub struct BuildFileAnalyzer {
    expand_cache: DashMap<Url, (i32, Arc<(String, sourcemap::SourceMap)>)>
}

impl BuildFileAnalyzer {
    pub fn new() -> Self {
        Self {
            expand_cache: DashMap::new()
        }
    }

    /// Drop `uri`'s cached expansion, if any — called on
    /// `textDocument/didClose` so a closed document's cache entry doesn't
    /// linger indefinitely.
    pub fn evict(&self, uri: &Url) {
        self.expand_cache.remove(uri);
    }
}

impl Default for BuildFileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
