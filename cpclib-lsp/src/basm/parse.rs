//! Shared parsing entry point: wraps the cpclib-asm parser for use by every
//! feature module.

use cpclib_asm::parser::context::ParserContextBuilder;
use cpclib_asm::parser::obtained::LocatedListing;
use cpclib_common::camino::Utf8PathBuf;
use tower_lsp::lsp_types::Url;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Parse the assembly document and return the listing
    pub(super) fn parse_document(
        &self,
        document: &Document
    ) -> Result<LocatedListing, LocatedListing> {
        Self::parse_source(&document.text(), Some(&document.uri))
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
