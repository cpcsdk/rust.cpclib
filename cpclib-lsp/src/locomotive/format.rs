//! Whole-document formatting (`textDocument/formatting`) for Locomotive
//! BASIC, built on `cpclib_basic::BasicProgram`'s own tokenizer/detokenizer:
//! parse the source into tokens, drop redundant space tokens, then re-render
//! via `Display` — which already normalizes keyword casing to the canonical
//! uppercase form. For on-type line-numbering, see `on_type_formatting.rs`.

use cpclib_basic::BasicProgram;
use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::document::Document;

impl BasicAnalyzer {
    /// `None` if the source doesn't parse as Locomotive BASIC, or is already
    /// in canonical form.
    pub fn format(&self, document: &Document) -> Option<Vec<TextEdit>> {
        let source = document.text();
        let mut program = BasicProgram::parse(&source).ok()?;
        program.remove_useless_space();
        let formatted = program.to_string();
        if formatted == source {
            return None;
        }

        let line_count = document.line_count() as u32;
        let last_line_len = document
            .line(line_count.saturating_sub(1) as usize)
            .map(|l| l.len() as u32)
            .unwrap_or(0);
        let whole_doc = Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: line_count,
                character: last_line_len
            }
        };
        Some(vec![TextEdit {
            range: whole_doc,
            new_text: formatted
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_text(text: &str) -> Option<String> {
        let uri = Url::parse("file:///t.bas").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        BasicAnalyzer::new()
            .format(&doc)
            .map(|edits| edits[0].new_text.clone())
    }

    #[test]
    fn normalizes_keyword_case() {
        assert_eq!(
            format_text("10 print \"A\"\n"),
            Some("10 PRINT \"A\"\n".to_string())
        );
    }

    #[test]
    fn trims_redundant_leading_and_trailing_spaces() {
        assert_eq!(format_text("10    CLS   \n"), Some("10 CLS\n".to_string()));
    }

    #[test]
    fn already_formatted_program_yields_no_edit() {
        assert_eq!(format_text("10 CLS\n20 PRINT\"A\"\n"), None);
    }

    #[test]
    fn unparsable_source_yields_no_edit() {
        assert_eq!(format_text("not valid basic at all {{{\n"), None);
    }
}
