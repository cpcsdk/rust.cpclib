//! Completion for Locomotive BASIC: keyword list rendered from the
//! documentation table in `token.rs`, plus variables already defined
//! elsewhere in the document. Keywords follow the case the user started
//! typing (lowercase prefix → lowercase completion).

use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::{KEYWORD_DOCS, collect_variable_occurrences};
use crate::common::document::Document;
use crate::common::render::first_doc_line;

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

        let mut completions: Vec<CompletionItem> = KEYWORD_DOCS
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
                    detail: Some(first_doc_line(doc)),
                    insert_text: Some(text),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.to_string()
                    })),
                    ..Default::default()
                }
            })
            .collect();

        completions.extend(self.variable_completions(document));

        completions
    }

    /// Variables already assigned somewhere in the document (`LET`/`FOR`
    /// targets, `INPUT`/`READ` targets, bare `NAME = ...`) — the same set
    /// the outline (`symbols.rs`) shows, so completion doesn't miss what the
    /// outline already knows about.
    fn variable_completions(&self, document: &Document) -> Vec<CompletionItem> {
        let Ok(prog) = self.parse_cached(document)
        else {
            return Vec::new();
        };

        collect_variable_occurrences(&prog)
            .into_values()
            .map(|(name, ..)| {
                CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    insert_text: Some(name),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    #[test]
    fn keyword_completions_carry_a_one_line_detail() {
        let uri = Url::parse("file:///t.bas").unwrap();
        let document = Document::new(uri, String::new(), 1);
        let items = BasicAnalyzer::new().completion(
            &document,
            Position {
                line: 0,
                character: 0
            }
        );
        let goto = items.iter().find(|i| i.label == "GOTO").unwrap();
        assert_eq!(
            goto.detail.as_deref(),
            Some("Unconditional jump to *line*.")
        );
    }

    #[test]
    fn short_doc_without_a_blank_line_still_yields_a_clean_detail() {
        let uri = Url::parse("file:///t.bas").unwrap();
        let document = Document::new(uri, String::new(), 1);
        let items = BasicAnalyzer::new().completion(
            &document,
            Position {
                line: 0,
                character: 0
            }
        );
        let then_item = items.iter().find(|i| i.label == "THEN").unwrap();
        // The doc has no "\n\n" break; the leading "**THEN**" marker must be
        // stripped rather than leaking into the completion menu's detail.
        assert!(
            !then_item
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("**")
        );
    }

    #[test]
    fn completion_proposes_variables_already_defined_in_the_document() {
        let uri = Url::parse("file:///t.bas").unwrap();
        let text = "10 LET SCORE = 0\n20 PRINT SCORE\n";
        let document = Document::new(uri, text.to_string(), 1);
        let items = BasicAnalyzer::new().completion(
            &document,
            Position {
                line: 1,
                character: 12
            }
        );
        let score = items
            .iter()
            .find(|i| i.label == "SCORE" && i.kind == Some(CompletionItemKind::VARIABLE));
        assert!(
            score.is_some(),
            "expected a SCORE variable completion, got {items:?}"
        );
    }
}
