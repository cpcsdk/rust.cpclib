//! LSP support for CSL (CPC Script Language) files (`.csl`): a
//! line-oriented emulator-automation script format, parsed by
//! `cpclib_csl`.
//!
//! Each feature lives in its own file; they all extend the same
//! `CslAnalyzer` type through separate `impl` blocks. Mirrors
//! `bndbuild::BuildFileAnalyzer`'s shape - the simplest existing analyzer,
//! since CSL, like bndbuild, needs no project/entry resolution the way
//! `.asm` does.

use std::sync::{Arc, RwLock};

use dashmap::DashMap;
use tower_lsp::lsp_types::Url;

use crate::common::config::CslConfig;

pub mod autocomplete;
pub mod command;
pub mod diagnostics;
pub mod run;

/// `parse_cache`'s value: `(document version, parse result)` - both the
/// `Ok` (clean parse) and `Err` (parse error, but still useful for
/// diagnostics) cases are cached, mirroring `basm::AssemblyAnalyzer::
/// parse_document`'s own reasoning.
type ParseCacheEntry = (i32, Result<Arc<cpclib_csl::CslScript>, Arc<cpclib_csl::CslError>>);

/// Analyzer for CSL files.
///
/// Feature implementations are spread across this module's files, one
/// `impl CslAnalyzer` block per concern.
pub struct CslAnalyzer {
    parse_cache: DashMap<Url, ParseCacheEntry>,
    /// Loaded once at `initialize()` - see `basm::AssemblyAnalyzer::config`'s
    /// own doc comment for the reasoning behind this shape.
    config: RwLock<Arc<CslConfig>>
}

impl CslAnalyzer {
    pub fn new() -> Self {
        Self {
            parse_cache: DashMap::new(),
            config: RwLock::new(Arc::new(CslConfig::default()))
        }
    }

    /// Drop `uri`'s cached parse, if any - called on
    /// `textDocument/didClose` so a closed document's cache entry doesn't
    /// linger indefinitely.
    pub fn evict(&self, uri: &Url) {
        self.parse_cache.remove(uri);
    }

    pub fn set_config(&self, config: CslConfig) {
        *self.config.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(config);
    }

    pub fn config(&self) -> Arc<CslConfig> {
        self.config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Parse `document`, reusing the cached result if it was already
    /// parsed at this exact `document.version`.
    pub(crate) fn parse_document(
        &self,
        document: &crate::common::document::Document
    ) -> Result<Arc<cpclib_csl::CslScript>, Arc<cpclib_csl::CslError>> {
        if let Some(entry) = self.parse_cache.get(&document.uri)
            && entry.0 == document.version
        {
            return entry.1.clone();
        }
        let text = document.text();
        let uri_path = document.uri.path().to_string();
        let result = cpclib_csl::parse_csl_with_rich_errors(&text, Some(uri_path))
            .map(Arc::new)
            .map_err(Arc::new);
        self.parse_cache
            .insert(document.uri.clone(), (document.version, result.clone()));
        result
    }
}

impl Default for CslAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
