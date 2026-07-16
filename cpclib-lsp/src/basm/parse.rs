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
        let text = document.text();

        // Create a parser context builder
        let builder = ParserContextBuilder::default();

        // Parse the assembly code using new_complete_source
        LocatedListing::new_complete_source(text, builder)
    }
}
