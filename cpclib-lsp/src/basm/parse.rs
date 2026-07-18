//! Shared parsing entry point: wraps the cpclib-asm parser for use by every
//! feature module.

use cpclib_asm::parser::context::ParserContextBuilder;
use cpclib_asm::parser::obtained::LocatedListing;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Parse the assembly document and return the listing
    pub(super) fn parse_document(
        &self,
        document: &Document
    ) -> Result<LocatedListing, LocatedListing> {
        Self::parse_source(&document.text())
    }

    /// Parse a raw assembly source string in isolation, with no `Document`
    /// required. Used directly by `diagnostics::analyze`'s multi-error
    /// recovery, which re-parses tail fragments of the file (everything
    /// after the last reported error) to surface more than just the first
    /// syntax error.
    pub(super) fn parse_source(text: &str) -> Result<LocatedListing, LocatedListing> {
        let builder = ParserContextBuilder::default();
        LocatedListing::new_complete_source(text, builder)
    }
}
