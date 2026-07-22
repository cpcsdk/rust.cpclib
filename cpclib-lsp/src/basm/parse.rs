//! Shared parsing entry point: wraps the cpclib-asm parser for use by every
//! feature module.

use std::sync::Arc;

use cpclib_asm::parser::context::ParserContextBuilder;
use cpclib_asm::parser::obtained::LocatedListing;
use cpclib_common::camino::Utf8PathBuf;
use tower_lsp::lsp_types::Url;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Parse the assembly document and return the listing, reusing the
    /// cached result if it was already parsed at this exact
    /// `document.version` - both the `Ok` (clean parse) and `Err`
    /// (parse-error, but still a usable partial listing for diagnostics)
    /// cases are cached, since even a broken file's parse can be expensive
    /// up to the error point and re-editing around a syntax error is a
    /// common, repeated case.
    pub(super) fn parse_document(
        &self,
        document: &Document
    ) -> Result<Arc<LocatedListing>, Arc<LocatedListing>> {
        if let Some(entry) = self.parse_cache.get(&document.uri)
            && entry.0 == document.version
        {
            return entry.1.clone();
        }
        let result = match Self::parse_source(&document.text(), Some(&document.uri)) {
            Ok(l) => Ok(Arc::new(l)),
            Err(l) => Err(Arc::new(l))
        };
        self.parse_cache
            .insert(document.uri.clone(), (document.version, result.clone()));
        result
    }

    /// Parse a raw assembly source string in isolation, with no `Document`
    /// required. Used directly by `diagnostics::analyze`'s multi-error
    /// recovery, which re-parses tail fragments of the file (everything
    /// after the last reported error) to surface more than just the first
    /// syntax error.
    ///
    /// `doc_uri`, when given, becomes the parser's `current_filename` — the
    /// LSP never set it before, so every basm error/diagnostic message
    /// showed the placeholder `"no file"`/`"no file specified"` instead of
    /// the real document path.
    pub(super) fn parse_source(
        text: &str,
        doc_uri: Option<&Url>
    ) -> Result<LocatedListing, LocatedListing> {
        // `quiet`: a `PRINT_PARSE` directive prints at *parse* time, before
        // any `Env`/`dry_run` exists to gate it — must be suppressed here
        // instead, since the LSP's real stdout carries JSON-RPC traffic.
        let mut builder = ParserContextBuilder::default().set_quiet(true);
        if let Some(path) = doc_uri
            .and_then(|uri| uri.to_file_path().ok())
            .and_then(|path| Utf8PathBuf::try_from(path).ok())
        {
            builder = builder.set_current_filename(path);
        }
        LocatedListing::new_complete_source(text, builder)
    }
}

#[cfg(test)]
mod parse_cache_tests {
    use super::*;

    fn doc(text: &str, version: i32) -> Document {
        Document::new(
            Url::parse("file:///t.asm").unwrap(),
            text.to_string(),
            version
        )
    }

    #[test]
    fn same_version_returns_the_identical_cached_arc() {
        let analyzer = AssemblyAnalyzer::new();
        let d = doc("start:\n    ret\n", 1);
        let first = analyzer.parse_document(&d).ok().unwrap();
        let second = analyzer.parse_document(&d).ok().unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_version_bump_reparses_and_returns_a_different_arc() {
        let analyzer = AssemblyAnalyzer::new();
        let d1 = doc("start:\n    ret\n", 1);
        let first = analyzer.parse_document(&d1).ok().unwrap();

        let d2 = doc("start:\n    ret\n    nop\n", 2);
        let second = analyzer.parse_document(&d2).ok().unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_parse_error_is_also_cached() {
        // A broken file's Err listing is still worth caching - re-editing
        // around a syntax error is a common, repeated case.
        let analyzer = AssemblyAnalyzer::new();
        let d = doc("@#$ garbage @#$\n", 1);
        let first = analyzer
            .parse_document(&d)
            .err()
            .expect("expected a parse error");
        let second = analyzer
            .parse_document(&d)
            .err()
            .expect("expected a parse error");
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn evict_forces_a_fresh_parse_even_at_the_same_version() {
        let analyzer = AssemblyAnalyzer::new();
        let d = doc("start:\n    ret\n", 1);
        let first = analyzer.parse_document(&d).ok().unwrap();
        analyzer.evict(&d.uri);
        let second = analyzer.parse_document(&d).ok().unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
    }
}
