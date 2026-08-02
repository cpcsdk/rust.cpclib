//! Inlay hints for assembly files: what a closing directive (`ENDIF`/
//! `ENDM`/...) or `ELSE`/`ELSEIF` belongs to, shown right after the token
//! itself, in the editor, without needing to hover - an explicit user
//! correction of an earlier hover-based attempt at this feature.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::{Document, byte_offset_to_utf16_col};

impl AssemblyAnalyzer {
    /// Every closing-directive/`ELSE`-family inlay hint whose own line
    /// falls within `range`.
    pub fn inlay_hints(&self, document: &Document, range: Range) -> Vec<InlayHint> {
        let text = document.text();
        let mut hints = Vec::new();

        for line in range.start.line..=range.end.line {
            let Some(line_text) = document.line(line as usize)
            else {
                continue;
            };
            let line_text = line_text.trim_end_matches(['\n', '\r']);
            let Some(opening_line) = super::token::matching_opening_line(&text, line)
            else {
                continue;
            };
            let Some(opening_text) = document.line(opening_line as usize)
            else {
                continue;
            };
            let end_char = byte_offset_to_utf16_col(line_text, line_text.len()) as u32;
            hints.push(InlayHint {
                position: Position {
                    line,
                    character: end_char
                },
                label: InlayHintLabel::String(format!("  // {}", opening_text.trim())),
                kind: None,
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: None
            });
        }
        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), text.to_string(), 1)
    }

    fn full_range(text: &str) -> Range {
        Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: text.lines().count() as u32,
                character: 0
            }
        }
    }

    #[test]
    fn endif_gets_a_hint_showing_the_opening_if() {
        let text = "if 1\n    nop\nendif\n";
        let d = doc(text);
        let hints = AssemblyAnalyzer::new().inlay_hints(&d, full_range(text));
        let endif_hint = hints
            .iter()
            .find(|h| h.position.line == 2)
            .expect("expected a hint on the endif line");
        match &endif_hint.label {
            InlayHintLabel::String(s) => assert!(s.contains("if 1"), "{s}"),
            _ => panic!("expected a string label")
        }
        // Positioned at the end of "endif" (5 chars).
        assert_eq!(endif_hint.position.character, 5);
    }

    #[test]
    fn else_gets_a_hint_showing_the_opening_if() {
        let text = "if 1\n    nop\nelse\n    nop\nendif\n";
        let d = doc(text);
        let hints = AssemblyAnalyzer::new().inlay_hints(&d, full_range(text));
        let else_hint = hints
            .iter()
            .find(|h| h.position.line == 2)
            .expect("expected a hint on the else line");
        match &else_hint.label {
            InlayHintLabel::String(s) => assert!(s.contains("if 1"), "{s}"),
            _ => panic!("expected a string label")
        }
    }

    #[test]
    fn ordinary_lines_get_no_hint() {
        let text = "if 1\n    nop\nendif\n";
        let d = doc(text);
        let hints = AssemblyAnalyzer::new().inlay_hints(&d, full_range(text));
        assert!(
            !hints
                .iter()
                .any(|h| h.position.line == 0 || h.position.line == 1)
        );
    }

    #[test]
    fn nested_endif_shows_its_own_matching_if_not_the_outer_one() {
        let text = "if 1\n    if 2\n        nop\n    endif\nendif\n";
        let d = doc(text);
        let hints = AssemblyAnalyzer::new().inlay_hints(&d, full_range(text));
        let inner = hints
            .iter()
            .find(|h| h.position.line == 3)
            .expect("expected a hint on the inner endif");
        match &inner.label {
            InlayHintLabel::String(s) => assert!(s.contains("if 2"), "{s}"),
            _ => panic!("expected a string label")
        }
    }
}
