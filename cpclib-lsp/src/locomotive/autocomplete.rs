//! Completion for Locomotive BASIC: keyword list rendered from the
//! documentation table in `token.rs`.

use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::KEYWORD_DOCS;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn completion(&self, _document: &Document, _position: Position) -> Vec<CompletionItem> {
        KEYWORD_DOCS
            .iter()
            .map(|(kw, doc)| {
                CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.to_string()
                    })),
                    ..Default::default()
                }
            })
            .collect()
    }
}
