//! Refactoring code-actions for assembly files: wrap in REPEAT/loop,
//! join statements onto one line, split multi-statement lines.

use cpclib_asm::unused_bindings::UnusedBindingKind;
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::command::{select_range_command, single_file_edit};
use super::format::{split_at_colon, strip_asm_comment};
use crate::common::document::{Document, byte_offset_to_utf16_col};

impl AssemblyAnalyzer {
    /// Offer to remove an unused REPEAT loop counter (`REPEAT 5, i` →
    /// `REPEAT 5`) when `range` (the requested code-action position - a
    /// plain cursor, i.e. `range.start == range.end`, in the overwhelmingly
    /// common case) falls on the declaring REPEAT header's own line.
    ///
    /// Re-derives the condition fresh from `(document, range)` at request
    /// time, the same way every other action in this file/`command.rs`
    /// already does - this codebase has no diagnostic↔code-action pairing
    /// mechanism (`CodeActionParams.context.diagnostics` is never read
    /// anywhere), so this doesn't invent one just for this quickfix.
    ///
    /// Only `RepeatCounter` bindings with a `removable_clause` are ever
    /// offered here: ITERATE/FOR counters are mandatory (nothing to
    /// remove), MACRO/FUNCTION parameter removal needs a call-site-aware
    /// rewrite (tracked separately, not built here - see the roadmap), and
    /// a REPEAT counter with an explicit start/step value is deliberately
    /// left warning-only too (see `removable_clause`'s own doc comment in
    /// `cpclib-asm`).
    pub(super) fn unused_repeat_counter_removal_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let (lo, hi) = (range.start.line, range.end.line);

        for binding in cpclib_asm::unused_bindings::find_unused_bindings(listing.iter()) {
            if binding.kind != UnusedBindingKind::RepeatCounter {
                continue;
            }
            let def_line = (binding.line as u32).saturating_sub(1);
            if def_line < lo || def_line > hi {
                continue;
            }
            let Some((clause_line, clause_column, clause_len)) = binding.removable_clause
            else {
                continue;
            };
            let line0 = (clause_line as u32).saturating_sub(1);
            let line_text = document.line(line0 as usize).unwrap_or_default();
            let byte_col = clause_column.saturating_sub(1);
            let start_char = byte_offset_to_utf16_col(&line_text, byte_col) as u32;
            let end_char = byte_offset_to_utf16_col(&line_text, byte_col + clause_len) as u32;
            let edit_range = Range {
                start: Position {
                    line: line0,
                    character: start_char
                },
                end: Position {
                    line: line0,
                    character: end_char
                }
            };
            return Some(CodeAction {
                title: format!("Remove unused loop counter '{}'", binding.name),
                kind: Some(CodeActionKind::QUICKFIX),
                edit: Some(single_file_edit(
                    document.uri.clone(),
                    edit_range,
                    String::new()
                )),
                ..Default::default()
            });
        }
        None
    }

    pub(super) fn wrap_action(
        &self,
        document: &Document,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
        header: &str,
        footer: &str,
        placeholder: &str,
        title: &str,
        kind: CodeActionKind
    ) -> CodeAction {
        // Detect minimum indentation of non-empty selected lines.
        let indent = lines[start_line..=end_line]
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // MACRO/ENDM always at column 0; body gets an extra \t when unindented.
        let mut new_text = format!("{header}\n");
        for &line in &lines[start_line..=end_line] {
            if indent == 0 {
                new_text.push('\t');
            }
            new_text.push_str(line.trim_end());
            new_text.push('\n');
        }
        new_text.push_str(&format!("{footer}\n"));

        let edit_range = Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        };

        // Select the placeholder text in the header line once the edit is applied,
        // so the user can immediately type a replacement (e.g. macro name / count).
        let command = header.find(placeholder).map(|col| {
            let header_line = start_line as u32;
            let placeholder_range = Range {
                start: Position {
                    line: header_line,
                    character: col as u32
                },
                end: Position {
                    line: header_line,
                    character: (col + placeholder.len()) as u32
                }
            };
            select_range_command(&document.uri, placeholder_range)
        });

        CodeAction {
            title: title.to_string(),
            kind: Some(kind),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            command,
            ..Default::default()
        }
    }

    pub(super) fn join_lines_action(
        &self,
        document: &Document,
        lines: &[&str],
        start_line: usize,
        end_line: usize
    ) -> Option<CodeAction> {
        // Indentation taken from the first non-empty line.
        let first = lines[start_line..=end_line]
            .iter()
            .find(|l| !l.trim().is_empty())?;
        let indent_len = first.len() - first.trim_start().len();
        let indent = &first[..indent_len];

        // Strip inline comments before joining so they don't eat subsequent parts.
        let parts: Vec<&str> = lines[start_line..=end_line]
            .iter()
            .map(|l| strip_asm_comment(l).trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() < 2 {
            return None;
        }

        let joined = format!("{}{}\n", indent, parts.join(" : "));
        let edit_range = Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        };
        Some(CodeAction {
            title: "Join selected lines (separate with :)".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, joined)),
            ..Default::default()
        })
    }

    pub(super) fn split_lines_action(
        &self,
        document: &Document,
        lines: &[&str],
        start_line: usize,
        end_line: usize
    ) -> Option<CodeAction> {
        let mut new_text = String::new();
        let mut any_split = false;

        for &line in &lines[start_line..=end_line] {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            let parts = split_at_colon(line);
            if parts.len() > 1 {
                any_split = true;
            }
            for part in parts {
                new_text.push_str(indent);
                new_text.push_str(part.trim_start());
                new_text.push('\n');
            }
        }

        if !any_split {
            return None;
        }

        let edit_range = Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        };
        Some(CodeAction {
            title: "Split lines at : (one instruction per line)".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod unused_repeat_counter_removal_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    #[test]
    fn offers_the_quickfix_with_a_correct_removal_edit_for_a_cursor_on_the_header_line() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .unused_repeat_counter_removal_action(&d, cursor(0, 3))
            .expect("expected the quickfix");
        assert_eq!(action.title, "Remove unused loop counter 'i'");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "");
        assert_eq!(
            text_edits[0].range,
            Range {
                start: Position {
                    line: 0,
                    character: 8
                },
                end: Position {
                    line: 0,
                    character: 11
                }
            }
        );
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(0, 3));
        assert!(actions.iter().any(|a| {
            a.title == "Remove unused loop counter 'i'" && a.kind == Some(CodeActionKind::QUICKFIX)
        }));
    }

    #[test]
    fn no_quickfix_when_the_repeat_counter_is_used() {
        let d = doc("REPEAT 5, i\n    db {i}\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_a_repeat_with_no_counter_at_all() {
        let d = doc("REPEAT 5\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_when_the_repeat_counter_has_an_explicit_start_value() {
        // Unused, but not offered: removing just ", i" here would silently
        // change behavior (dropping the "10" start value along with it), so
        // this stays warning-only, matching `removable_clause`'s own
        // documented scope in cpclib-asm.
        let d = doc("REPEAT 5, i, 10\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_iterate_counter() {
        let d = doc("ITERATE i, [1, 2, 3]\n    nop\nENDITERATE\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_for_counter() {
        let d = doc("FOR i, 0, 5, 1\n    nop\nENDFOR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_macro_parameter() {
        let d = doc("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_function_parameter() {
        let d = doc("FUNCTION f, a, b\n    RETURN {a}\nENDFUNCTION\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn does_not_trigger_when_the_cursor_is_on_an_unrelated_line() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n\nld a, b\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(4, 0))
                .is_none()
        );
    }

    #[test]
    fn triggers_for_a_selection_spanning_the_header_line_not_just_a_collapsed_cursor() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        let range = Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: 2,
                character: 4
            }
        };
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, range)
                .is_some()
        );
    }
}
