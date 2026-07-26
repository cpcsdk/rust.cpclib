//! Code actions dispatch and LSP command/edit construction helpers for
//! assembly files. The individual refactorings live in `refactor.rs`.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::cycles::{self, SelectionCycleCount};
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

        // Offer removal of an unused REPEAT loop counter when the cursor/
        // selection sits on its declaring header line - checked before the
        // real-selection gate below since this must also work for a plain
        // cursor position (the common way a quickfix gets triggered), not
        // just an explicit multi-line selection.
        if let Some(a) = self.unused_repeat_counter_removal_action(document, range) {
            actions.push(a);
        }

        let Some((start_line, end_line)) = line_range_from_selection(range, all_lines.len())
        else {
            return actions;
        };

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

        // Cycle count for the selection - an intentionally informational,
        // no-op action (no `edit`, no `command`): the answer is the title
        // itself, shown right in the Quick Fix menu. There's no better
        // existing mechanism in this codebase for "just show a computed
        // value tied to a selection" than this established trick.
        if let Some(summary) = self.cycle_count_for_selection(document, range) {
            actions.push(CodeAction {
                title: cycles::format_title(&summary),
                kind: Some(CodeActionKind::EMPTY),
                edit: None,
                command: None,
                ..Default::default()
            });
        }

        actions
    }

    /// Total NOP count for the instructions in `range`, or `None` when
    /// there's no real selection or nothing recognizable in it. Shared by
    /// the `code_actions` Quick Fix entry above and the
    /// `cpclib.cycleCountForSelection` command (`backend.rs`) that drives
    /// the VS Code status-bar live display.
    pub fn cycle_count_for_selection(
        &self,
        document: &Document,
        range: Range
    ) -> Option<SelectionCycleCount> {
        let text = document.text();
        let all_lines: Vec<&str> = text.lines().collect();
        let (start_line, end_line) = line_range_from_selection(range, all_lines.len())?;
        let summary = cycles::count_cycles_in_lines(&all_lines, start_line, end_line);
        if summary.is_empty() {
            None
        }
        else {
            Some(summary)
        }
    }
}

/// `range` (an LSP selection) to an inclusive `(start_line, end_line)` pair,
/// or `None` when there's no real selection (`range.start == range.end`) or
/// it's empty/inverted after clamping to `line_count`. Handles the
/// "`end.line` is exclusive when `character == 0`" LSP quirk.
pub(super) fn line_range_from_selection(range: Range, line_count: usize) -> Option<(usize, usize)> {
    if range.start == range.end {
        return None;
    }
    let start_line = range.start.line as usize;
    let end_line = if range.end.character == 0 && range.end.line > range.start.line {
        (range.end.line as usize).saturating_sub(1)
    }
    else {
        range.end.line as usize
    }
    .min(line_count.saturating_sub(1));

    if start_line > end_line {
        None
    }
    else {
        Some((start_line, end_line))
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

#[cfg(test)]
mod cycle_count_action_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn full_line_range(start: u32, end: u32) -> Range {
        Range {
            start: Position {
                line: start,
                character: 0
            },
            end: Position {
                line: end,
                character: 0
            }
        }
    }

    #[test]
    fn the_action_appears_with_the_right_title_for_a_real_selection() {
        let d = doc("    ld a,b\n    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 2));
        let cycle_action = actions
            .iter()
            .find(|a| a.title.starts_with("Cycle count:"))
            .expect("expected a cycle-count action");
        assert_eq!(cycle_action.title, "Cycle count: 2 NOPs");
        assert!(cycle_action.edit.is_none());
        assert!(cycle_action.command.is_none());
        assert_eq!(cycle_action.kind, Some(CodeActionKind::EMPTY));
    }

    #[test]
    fn no_cycle_count_action_without_a_selection() {
        let d = doc("    ld a,b\n    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let collapsed = Range {
            start: Position {
                line: 0,
                character: 4
            },
            end: Position {
                line: 0,
                character: 4
            }
        };
        let actions = analyzer.code_actions(&d, collapsed);
        assert!(!actions.iter().any(|a| a.title.starts_with("Cycle count:")));
    }

    #[test]
    fn no_cycle_count_action_when_the_selection_has_nothing_recognizable() {
        let d = doc("    ; just a comment\n\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 2));
        assert!(!actions.iter().any(|a| a.title.starts_with("Cycle count:")));
    }

    #[test]
    fn cycle_count_for_selection_returns_none_for_a_collapsed_range() {
        let d = doc("    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let collapsed = Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: 0,
                character: 0
            }
        };
        assert!(analyzer.cycle_count_for_selection(&d, collapsed).is_none());
    }
}
