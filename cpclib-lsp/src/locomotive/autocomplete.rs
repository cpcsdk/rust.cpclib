//! Completion for Locomotive BASIC: keyword list rendered from the
//! documentation table in `token.rs`. Keywords follow the case the user
//! started typing (lowercase prefix → lowercase completion).

use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::KEYWORD_DOCS;
use crate::common::document::Document;

impl BasicAnalyzer {
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        // Respect the case of what the user already typed.
        let lowercase = document
            .line(position.line as usize)
            .map(|line| {
                let bytes = line.as_bytes();
                let col = (position.character as usize).min(bytes.len());
                let mut start = col;
                while start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
                    start -= 1;
                }
                bytes[start..col]
                    .iter()
                    .find(|b| b.is_ascii_alphabetic())
                    .is_some_and(|b| b.is_ascii_lowercase())
            })
            .unwrap_or(false);

        KEYWORD_DOCS
            .iter()
            .map(|(kw, doc)| {
                let text = if lowercase {
                    kw.to_lowercase()
                }
                else {
                    kw.to_string()
                };
                CompletionItem {
                    label: text.clone(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    insert_text: Some(text),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
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
