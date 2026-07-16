//! Refactoring code-actions for assembly files: wrap in REPEAT/loop,
//! join statements onto one line, split multi-statement lines.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::command::{select_range_command, single_file_edit};
use super::format::{split_at_colon, strip_asm_comment};
use crate::common::document::Document;

impl AssemblyAnalyzer {
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
