//! Formatting for assembly files (via cpclib-asmfmt) and the
//! statement-splitting text helpers it shares with the refactorings.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// or `None` if the source cannot be parsed or is already correctly formatted.
    /// `opt` is the already-resolved format options (config loading is the caller's responsibility
    /// so the backend can report config errors to the LSP client).
    pub fn format(
        &self,
        document: &Document,
        opt: &cpclib_asmfmt::AsmFormatOptions
    ) -> Option<Vec<TextEdit>> {
        let source = document.text();
        let formatted = cpclib_asmfmt::format(&source, opt).ok()?;
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

    /// Delegates to BASIC on-type line-numbering when `position` falls
    /// inside a `LOCOMOTIVE`/`ENDLOCOMOTIVE` block — basm files have no
    /// on-type formatting of their own.
    pub fn on_type_newline(
        &self,
        document: &Document,
        position: Position
    ) -> Option<Vec<TextEdit>> {
        let text = document.text();
        let line_idx = position.line as usize;
        let (block, basic_text) = super::embedded_basic::block_and_text_at(&text, line_idx)?;
        crate::locomotive::on_type_formatting::locomotive_basic_on_type_newline(
            &basic_text,
            position,
            block.basic_range.start as u32
        )
    }
}

/// Strip a trailing `;`-comment from an ASM line (string-literal aware).
/// Returns the slice up to (but not including) the `;`.
pub(super) fn strip_asm_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_str = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_str = !in_str,
            b';' if !in_str => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Split an ASM line at `:` statement separators (string-literal aware).
/// A `:` that immediately follows a bare identifier (label colon) is NOT split.
/// Split `line` at top-level (non-string) single `:` characters, one
/// instruction per part. `::` is a label-reference prefix (e.g. `::foo`),
/// not a statement separator, and is never split. A part that is a bare
/// label identifier (e.g. `loop`) is re-suffixed with `:` so it becomes its
/// own `loop:` line instead of staying glued to the following instruction.
pub(super) fn split_at_colon(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut raw_parts: Vec<&str> = Vec::new();
    let mut in_str = false;
    let mut start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                in_str = !in_str;
                i += 1;
            },
            b':' if !in_str => {
                if i + 1 < bytes.len() && bytes[i + 1] == b':' {
                    i += 2;
                    continue;
                }
                raw_parts.push(&line[start..i]);
                start = i + 1;
                i += 1;
            },
            _ => {
                i += 1;
            }
        }
    }
    raw_parts.push(&line[start..]);

    let n = raw_parts.len();
    raw_parts
        .into_iter()
        .enumerate()
        .filter_map(|(idx, part)| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            // A part followed by a colon in the source, made up only of
            // identifier characters, is a label definition.
            let is_label = idx + 1 < n
                && trimmed
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '@');
            Some(if is_label {
                format!("{trimmed}:")
            }
            else {
                trimmed.to_string()
            })
        })
        .collect()
}

#[cfg(test)]
mod on_type_newline_tests {
    use super::*;
    use crate::common::document::Document;

    #[test]
    fn continues_numbering_inside_a_locomotive_block() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "ORG 0x8000\nLOCOMOTIVE\n10 PRINT \"A\"\n\nENDLOCOMOTIVE\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on the blank new line 3, right after "10 PRINT \"A\"".
        let edits = AssemblyAnalyzer::new()
            .on_type_newline(
                &doc,
                Position {
                    line: 3,
                    character: 0
                }
            )
            .expect("expected a line-numbering edit inside the LOCOMOTIVE block");
        assert_eq!(edits[0].new_text, "20 ");
        assert_eq!(edits[0].range.start.line, 3);
    }

    #[test]
    fn outside_any_locomotive_block_yields_no_edit() {
        let uri = Url::parse("file:///t.asm").unwrap();
        let text = "ORG 0x8000\n\n";
        let doc = Document::new(uri, text.to_string(), 1);
        assert!(
            AssemblyAnalyzer::new()
                .on_type_newline(
                    &doc,
                    Position {
                        line: 1,
                        character: 0
                    }
                )
                .is_none()
        );
    }
}

#[cfg(test)]
mod split_at_colon_tests {
    use super::split_at_colon;

    #[test]
    fn splits_label_onto_its_own_line() {
        let parts = split_at_colon(".loop: ld a,(hl) : inc hl : djnz .loop");
        assert_eq!(parts, vec![".loop:", "ld a,(hl)", "inc hl", "djnz .loop"]);
    }

    #[test]
    fn no_label_just_splits_instructions() {
        let parts = split_at_colon("ld a,1 : ld b,2");
        assert_eq!(parts, vec!["ld a,1", "ld b,2"]);
    }

    #[test]
    fn double_colon_is_not_a_separator() {
        let parts = split_at_colon("ld hl,::foo : ret");
        assert_eq!(parts, vec!["ld hl,::foo", "ret"]);
    }

    #[test]
    fn colon_inside_string_is_preserved() {
        let parts = split_at_colon("ld a,\"x:y\" : ret");
        assert_eq!(parts, vec!["ld a,\"x:y\"", "ret"]);
    }

    #[test]
    fn trailing_label_with_no_following_instruction() {
        let parts = split_at_colon(".end:");
        assert_eq!(parts, vec![".end:"]);
    }
}
