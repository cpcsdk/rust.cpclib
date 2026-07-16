//! Code actions dispatch and LSP command/edit construction helpers for
//! assembly files. The individual refactorings live in `refactor.rs`.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::extract_locomotive_blocks;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    // ── Code actions ──────────────────────────────────────────────────────────

    pub fn code_actions(&self, document: &Document, range: Range) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        let text = document.text();
        let all_lines: Vec<&str> = text.lines().collect();

        // Offer RENUM when cursor/selection is inside a LOCOMOTIVE block.
        let loco_blocks = extract_locomotive_blocks(&text);
        let cursor_line = range.start.line as usize;
        if let Some(block) = loco_blocks
            .iter()
            .find(|b| b.basic_range.contains(&cursor_line))
        {
            let basic_text: String = block
                .basic_range
                .clone()
                .filter_map(|i| all_lines.get(i).copied())
                .collect::<Vec<_>>()
                .join("\n");
            if let Ok(new_basic) = cpclib_basic::renum::renum_text(&basic_text, 10, 10) {
                if new_basic != basic_text {
                    let new_text = if new_basic.ends_with('\n') {
                        new_basic
                    }
                    else {
                        format!("{new_basic}\n")
                    };
                    let edit_range = Range {
                        start: Position {
                            line: block.basic_range.start as u32,
                            character: 0
                        },
                        end: Position {
                            line: block.basic_range.end as u32,
                            character: 0
                        }
                    };
                    actions.push(CodeAction {
                        title: "Renumber BASIC lines in LOCOMOTIVE block (10, 20, 30…)".to_string(),
                        kind: Some(CodeActionKind::REFACTOR_REWRITE),
                        edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
                        ..Default::default()
                    });
                }
            }
        }

        let has_selection = range.start != range.end;
        if !has_selection {
            return actions;
        }
        let start_line = range.start.line as usize;
        // end.line is exclusive when character == 0; include last non-empty line
        let end_line = if range.end.character == 0 && range.end.line > range.start.line {
            (range.end.line as usize).saturating_sub(1)
        }
        else {
            range.end.line as usize
        }
        .min(all_lines.len().saturating_sub(1));

        if start_line > end_line {
            return actions;
        }

        // Wrap in MACRO / ENDM
        actions.push(self.wrap_action(
            document,
            &all_lines,
            start_line,
            end_line,
            "MACRO MY_MACRO",
            "ENDM",
            "MY_MACRO",
            "Wrap selection in MACRO…ENDM (rename MY_MACRO)",
            CodeActionKind::REFACTOR_EXTRACT
        ));

        // Wrap in REPEAT / REND
        actions.push(self.wrap_action(
            document,
            &all_lines,
            start_line,
            end_line,
            "REPEAT 10",
            "REND",
            "10",
            "Wrap selection in REPEAT…REND (replace 10 with count)",
            CodeActionKind::REFACTOR_EXTRACT
        ));

        // Join selected lines into one (instructions separated by " : ")
        if end_line > start_line {
            if let Some(a) = self.join_lines_action(document, &all_lines, start_line, end_line) {
                actions.push(a);
            }
        }

        // Split each line at " : " into individual lines
        if let Some(a) = self.split_lines_action(document, &all_lines, start_line, end_line) {
            actions.push(a);
        }

        actions
    }
}

// ─── Code-action helpers ──────────────────────────────────────────────────────

/// Build a `WorkspaceEdit` that replaces one range in one file.
pub(super) fn single_file_edit(uri: Url, range: Range, new_text: String) -> WorkspaceEdit {
    WorkspaceEdit {
        changes: Some(std::collections::HashMap::from([(
            uri,
            vec![TextEdit { range, new_text }]
        )])),
        ..Default::default()
    }
}

/// Build a `cpclib.selectRange` command that, once the code action's edit has
/// been applied, asks the client (via `window/showDocument`) to select
/// `range` so the user can immediately type a replacement.
pub(super) fn select_range_command(uri: &Url, range: Range) -> Command {
    Command {
        title: "Select placeholder".to_string(),
        command: "cpclib.selectRange".to_string(),
        arguments: Some(vec![serde_json::json!({
            "uri": uri.to_string(),
            "range": range,
        })])
    }
}
