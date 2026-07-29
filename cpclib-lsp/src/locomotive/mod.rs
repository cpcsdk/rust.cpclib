//! LSP support for Locomotive BASIC (`.bas` files), also reused by the basm
//! module for BASIC blocks embedded in Z80 assembly (`LOCOMOTIVE` directive).
//!
//! Each feature lives in its own file; they all extend the same
//! `BasicAnalyzer` type through separate `impl` blocks.

use std::sync::{Arc, RwLock};

use cpclib_basic::BasicError;
use cpclib_basic::located::LocatedBasicProgram;
use dashmap::DashMap;
use tower_lsp::lsp_types::Url;

use crate::common::config::BasicConfig;
use crate::common::document::Document;

pub mod autocomplete;
pub mod call_hierarchy;
pub mod catart;
pub mod color;
pub mod command;
pub mod definition;
pub mod diagnostics;
pub mod format;
pub mod hover;
pub mod on_type_formatting;
pub mod run;
pub mod semantic_tokens;
pub mod symbols;
pub mod token;

/// Analyzer for Locomotive BASIC documents.
///
/// Feature implementations are spread across this module's files, one
/// `impl BasicAnalyzer` block per concern.
///
/// Holds a parse-result cache keyed by document URI: `LocatedBasicProgram`
/// is fully owned plain data (no borrowed source text, unlike basm's
/// self-referential `LocatedListing`), so it's safe to `Arc`-share across
/// requests with no unsafe code - this avoids every feature (hover,
/// semantic tokens, document symbols, diagnostics, call hierarchy, ...)
/// independently re-lexing/parsing the same document text on every single
/// request.
pub struct BasicAnalyzer {
    parse_cache: DashMap<Url, (i32, Arc<LocatedBasicProgram>)>,
    /// Loaded once at `initialize()` - see `AssemblyAnalyzer::config`'s own
    /// doc comment for the reasoning behind this shape.
    config: RwLock<Arc<BasicConfig>>
}

impl BasicAnalyzer {
    pub fn new() -> Self {
        Self {
            parse_cache: DashMap::new(),
            config: RwLock::new(Arc::new(BasicConfig::default()))
        }
    }

    pub fn set_config(&self, config: BasicConfig) {
        *self.config.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(config);
    }

    pub fn config(&self) -> Arc<BasicConfig> {
        self.config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Parse `document`'s BASIC source, reusing the cached result if it was
    /// already parsed at this exact `document.version`. Parse errors are
    /// never cached (cheap to reproduce, and avoids needing `BasicError:
    /// Clone`) - only successful parses are stored.
    pub(super) fn parse_cached(
        &self,
        document: &Document
    ) -> Result<Arc<LocatedBasicProgram>, BasicError> {
        if let Some(entry) = self.parse_cache.get(&document.uri)
            && entry.0 == document.version
        {
            return Ok(Arc::clone(&entry.1));
        }
        let prog = Arc::new(LocatedBasicProgram::parse(&document.text())?);
        self.parse_cache
            .insert(document.uri.clone(), (document.version, Arc::clone(&prog)));
        Ok(prog)
    }

    /// Drop `uri`'s cached parse, if any - called on `textDocument/didClose`
    /// so a closed document's cache entry doesn't linger indefinitely.
    pub(super) fn evict(&self, uri: &Url) {
        self.parse_cache.remove(uri);
    }
}

impl Default for BasicAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod parse_cache_tests {
    use super::*;

    fn doc(text: &str, version: i32) -> Document {
        Document::new(
            Url::parse("file:///t.bas").unwrap(),
            text.to_string(),
            version
        )
    }

    #[test]
    fn same_version_returns_the_identical_cached_arc() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 PRINT 1\n", 1);
        let first = analyzer.parse_cached(&d).unwrap();
        let second = analyzer.parse_cached(&d).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_version_bump_reparses_and_returns_a_different_arc() {
        let analyzer = BasicAnalyzer::new();
        let d1 = doc("10 PRINT 1\n", 1);
        let first = analyzer.parse_cached(&d1).unwrap();

        let d2 = doc("10 PRINT 2\n", 2);
        let second = analyzer.parse_cached(&d2).unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
        // Content actually reflects the new version, not a stale cache hit.
        let number_tok = second.lines[0]
            .tokens
            .iter()
            .find_map(|t| {
                match &t.kind {
                    cpclib_basic::located::LocatedTokenKind::Number(n) => Some(n.as_str()),
                    _ => None
                }
            })
            .expect("expected a Number token");
        assert_eq!(number_tok, "2");
    }

    #[test]
    fn evict_forces_a_fresh_parse_even_at_the_same_version() {
        let analyzer = BasicAnalyzer::new();
        let d = doc("10 PRINT 1\n", 1);
        let first = analyzer.parse_cached(&d).unwrap();
        analyzer.evict(&d.uri);
        let second = analyzer.parse_cached(&d).unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
    }
}
