//! Code actions dispatch and LSP command/edit construction helpers for
//! assembly files. The individual refactorings live in `refactor.rs`.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::cycles::{self, SelectionCycleCount};
use super::embedded_basic::extract_locomotive_blocks;
use super::refactor::WrapActionSpec;
use super::registers::AllRegisters;
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
            if let Ok(new_basic) = cpclib_basic::renum::renum_text(&basic_text, 10, 10)
                && new_basic != basic_text
            {
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

        // Offer removal of an unused REPEAT loop counter, or an unused
        // MACRO/FUNCTION parameter, when the cursor/selection sits on its
        // declaring header line - checked before the real-selection gate
        // below since this must also work for a plain cursor position (the
        // common way a quickfix gets triggered), not just an explicit
        // multi-line selection.
        if let Some(a) = self.unused_repeat_counter_removal_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.unused_macro_or_function_parameter_removal_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.redundant_accumulator_prefix_removal_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.no_op_or_improvable_instruction_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.smc_label_equ_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.fake_instruction_to_real_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.firmware_symbol_replacement_action(document, range) {
            actions.push(a);
        }
        if let Some(a) = self.peephole_quickfix_action(document, range) {
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
            WrapActionSpec {
                header: "MACRO MY_MACRO",
                footer: "ENDM",
                placeholder: "MY_MACRO",
                title: "Wrap selection in MACRO…ENDM (rename MY_MACRO)",
                kind: CodeActionKind::REFACTOR_EXTRACT
            }
        ));

        // Wrap in REPEAT / REND
        actions.push(self.wrap_action(
            document,
            &all_lines,
            start_line,
            end_line,
            WrapActionSpec {
                header: "REPEAT 10",
                footer: "REND",
                placeholder: "10",
                title: "Wrap selection in REPEAT…REND (replace 10 with count)",
                kind: CodeActionKind::REFACTOR_EXTRACT
            }
        ));

        // Join selected lines into one (instructions separated by " : ")
        if end_line > start_line
            && let Some(a) = self.join_lines_action(document, &all_lines, start_line, end_line)
        {
            actions.push(a);
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

        // Balance branch timings in the selection - pads every runtime path
        // through a hand-written JR/JP/RET branch so they all cost the
        // same. Offered only on success: a selection that doesn't fit this
        // v1's supported shape (a loop, DJNZ/CALL, an escaping jump target,
        // a conditional RET needing a rewrite with no qualifying global
        // label in view, an unparseable document, ...) isn't a mistake the
        // user made, so it gets no action at all rather than an error
        // popup - same "nothing recognizable -> no entry" precedent as the
        // cycle-count action above.
        if let Ok(text_edits) = self.stabilize_text_edits(document, start_line, end_line) {
            actions.push(CodeAction {
                title: "Balance branch timings in selection".to_string(),
                kind: Some(CodeActionKind::REFACTOR_REWRITE),
                edit: Some(single_file_multi_edit(document.uri.clone(), text_edits)),
                ..Default::default()
            });
        }

        actions
    }

    /// The `TextEdit`s that balance the branch timings of whole lines
    /// `start_line..=end_line`, or the reason there are none: the document
    /// doesn't parse, the selection holds something this feature can't
    /// balance (see `stabilize.rs`), or it's already balanced.
    ///
    /// Shared by the Refactor code action - which offers nothing at all on
    /// `Err`, since an unsupported shape isn't a mistake the user made - and
    /// the `cpclib.stabilizeEdits` command behind the editor's explicit
    /// "Stabilize Selection", which is the one place that shows the reason.
    pub fn stabilize_text_edits(
        &self,
        document: &Document,
        start_line: usize,
        end_line: usize
    ) -> Result<Vec<TextEdit>, String> {
        let text = document.text();
        let all_lines: Vec<&str> = text.lines().collect();
        let listing = self
            .parse_document(document)
            .map_err(|_| "the document doesn't parse".to_string())?;
        let edits = super::stabilize::stabilize_lines(&listing, start_line, end_line)?;
        if edits.is_empty() {
            return Err(
                "nothing to balance: the selection has no branch whose two paths differ in \
                 timing (a branch with a loop or a jump out of the selection on one of its arms \
                 is left alone)"
                    .to_string()
            );
        }
        let text_edits: Vec<TextEdit> = edits
            .into_iter()
            .map(|edit| {
                match edit {
                    super::stabilize::StabilizeTextEdit::InsertPadding { line, nop_count } => {
                        let pos = Position {
                            line: line as u32,
                            character: 0
                        };
                        TextEdit {
                            range: Range {
                                start: pos,
                                end: pos
                            },
                            new_text: format!("    waitnops {nop_count}\n")
                        }
                    },
                    super::stabilize::StabilizeTextEdit::InsertText { line, text } => {
                        let pos = Position {
                            line: line as u32,
                            character: 0
                        };
                        TextEdit {
                            range: Range {
                                start: pos,
                                end: pos
                            },
                            new_text: text
                        }
                    },
                    super::stabilize::StabilizeTextEdit::ReplaceRetWithJump {
                        range,
                        new_text
                    } => TextEdit { range, new_text },
                    super::stabilize::StabilizeTextEdit::AppendTailBlocks {
                        at,
                        text: append_text
                    } => {
                        // If `at` doesn't correspond to any real
                        // existing line (past the document's own last
                        // line) and the document has no trailing
                        // newline, there is no real line break for `at`
                        // to anchor to - inserting there would glue
                        // straight onto the end of the last line's own
                        // text instead of starting a fresh line. Anchor
                        // to the end of that last line's own content
                        // instead and supply the leading newline
                        // ourselves.
                        let past_last_line = at.line as usize >= all_lines.len();
                        let needs_own_newline = past_last_line && !text.ends_with('\n');
                        let (pos, new_text) = if needs_own_newline {
                            let last_line = all_lines.len().saturating_sub(1);
                            let end_char =
                                all_lines.get(last_line).map_or(0, |l| l.len()) as u32;
                            (
                                Position {
                                    line: last_line as u32,
                                    character: end_char
                                },
                                format!("\n{append_text}")
                            )
                        }
                        else {
                            (at, append_text)
                        };
                        TextEdit {
                            range: Range {
                                start: pos,
                                end: pos
                            },
                            new_text
                        }
                    }
                }
            })
            .collect();
        Ok(text_edits)
    }

    /// Total NOP count for the instructions in `range`, or `None` when
    /// there's nothing recognizable in range. Shared by the `code_actions`
    /// Quick Fix entry above (which only ever calls this for a real
    /// selection - its own, separate `line_range_from_selection` check
    /// happens first and returns early otherwise, so this function's own
    /// bare-cursor handling below never affects the Quick Fix menu) and
    /// the `cpclib.cycleCountForSelection` command (`backend.rs`) that
    /// drives the VS Code status-bar live display, which calls this for
    /// *every* cursor move, selected or not.
    ///
    /// A bare cursor position (`range.start == range.end`, no real
    /// selection) shows the cost of just the single line it's on, rather
    /// than nothing at all - the common case is one instruction per line,
    /// so this reads as "the cost of the instruction under the cursor"; a
    /// `:`-chained multi-instruction line shows their combined cost
    /// instead of isolating just the one nearest the cursor, a deliberate
    /// simplification (matches how a real selection spanning that same
    /// line would already behave).
    pub fn cycle_count_for_selection(
        &self,
        document: &Document,
        range: Range
    ) -> Option<SelectionCycleCount> {
        let text = document.text();
        let all_lines: Vec<&str> = text.lines().collect();
        let (start_line, end_line) = if range.start == range.end {
            if all_lines.is_empty() {
                return None;
            }
            let line = (range.start.line as usize).min(all_lines.len() - 1);
            (line, line)
        }
        else {
            line_range_from_selection(range, all_lines.len())?
        };
        let listing = self.parse_document(document).ok()?;
        let summary = cycles::count_cycles_in_lines(&listing, start_line, end_line);
        if summary.is_empty() {
            None
        }
        else {
            Some(summary)
        }
    }

    /// Every tracked register's statically-known value at `position` (the
    /// same 13 registers the single-register hover already tracks) - `None`
    /// only when the document doesn't parse at all. Backs the VS Code
    /// "all registers" status bar item (`cpclib.registersAtPosition`,
    /// `backend.rs`), the all-at-once counterpart to hover's own
    /// per-register `registers::register_state_at` lookup.
    pub fn all_registers_at(
        &self,
        document: &Document,
        position: Position
    ) -> Option<AllRegisters> {
        let listing = self.parse_document(document).ok()?;
        let mut env = self.local_symbols_env_cached(document, &listing);
        Some(super::registers::all_tracked_registers_at(
            &listing, &mut env, position
        ))
    }
}

/// `range` (an LSP selection) to an inclusive `(start_line, end_line)` pair,
/// or `None` when there's no real selection (`range.start == range.end`) or
/// it's empty/inverted after clamping to `line_count`. Handles the
/// "`end.line` is exclusive when `character == 0`" LSP quirk.
pub(crate) fn line_range_from_selection(range: Range, line_count: usize) -> Option<(usize, usize)> {
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
    single_file_multi_edit(uri, vec![TextEdit { range, new_text }])
}

/// As [`single_file_edit`], but for several non-overlapping edits in one
/// document (e.g. NOP padding inserted at more than one point by the
/// branch-timing-stabilization action) - `TextEdit`s are resolved against
/// the *original* document, so the caller doesn't need to worry about
/// later edits shifting earlier ones' positions.
pub(super) fn single_file_multi_edit(uri: Url, edits: Vec<TextEdit>) -> WorkspaceEdit {
    WorkspaceEdit {
        changes: Some(std::collections::HashMap::from([(uri, edits)])),
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

/// Build a `cpclib.removeUnusedParameter` command - unlike every other
/// action in this file/`refactor.rs`, this one has no synchronous `edit` at
/// all: finding every call site across the workspace is real I/O (parsing
/// every candidate file), too expensive to do on every `codeAction` request
/// (which fires on essentially every cursor move in many clients). The
/// command name is reached only from here (no client-side contribution
/// needed, mirroring `cpclib.selectRange`'s own precedent) - the actual
/// cross-file work happens in `execute_command` (`server/backend.rs`) once
/// the user picks this from the Quick Fix menu.
pub(super) fn remove_unused_parameter_command(
    uri: &Url,
    kind: crate::basm::remove_parameter::RemoveParameterKind,
    owner_name: &str,
    param_index: usize
) -> Command {
    let kind_str = match kind {
        crate::basm::remove_parameter::RemoveParameterKind::Macro => "macro",
        crate::basm::remove_parameter::RemoveParameterKind::Function => "function"
    };
    Command {
        title: "Remove unused parameter".to_string(),
        command: "cpclib.removeUnusedParameter".to_string(),
        arguments: Some(vec![serde_json::json!({
            "uri": uri.to_string(),
            "kind": kind_str,
            "ownerName": owner_name,
            "paramIndex": param_index,
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
    fn cycle_count_for_selection_shows_the_cursor_lines_own_cost_with_no_real_selection() {
        // A bare cursor position (no selection) now shows the cost of the
        // single line it's on, rather than nothing at all - the status
        // bar should stay live as the cursor moves, not just while
        // dragging out an actual selection.
        let d = doc("    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let collapsed = Range {
            start: Position {
                line: 0,
                character: 2
            },
            end: Position {
                line: 0,
                character: 2
            }
        };
        let summary = analyzer
            .cycle_count_for_selection(&d, collapsed)
            .expect("expected a summary for the cursor's own line");
        assert_eq!(summary.min_nops, 1, "{summary:?}");
        assert_eq!(summary.max_nops, 1, "{summary:?}");
    }

    #[test]
    fn cycle_count_for_selection_returns_none_when_the_cursor_line_has_nothing_recognizable() {
        let d = doc("    ; just a comment\n    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let collapsed = Range {
            start: Position {
                line: 0,
                character: 5
            },
            end: Position {
                line: 0,
                character: 5
            }
        };
        assert!(analyzer.cycle_count_for_selection(&d, collapsed).is_none());
    }

    #[test]
    fn all_registers_at_reports_the_known_value_at_the_cursor() {
        let d = doc("    ld a,5\n    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let regs = analyzer
            .all_registers_at(
                &d,
                Position {
                    line: 1,
                    character: 4
                }
            )
            .expect("expected a document that parses");
        assert_eq!(regs.a.as_deref(), Some("0x05"));
        assert_eq!(regs.hl, None);
    }
}

#[cfg(test)]
mod stabilize_branch_timings_action_tests {
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

    fn stabilize_action(actions: &[CodeAction]) -> Option<&CodeAction> {
        actions
            .iter()
            .find(|a| a.title.starts_with("Balance branch timings"))
    }

    #[test]
    fn the_action_appears_and_inserts_nops_for_an_unbalanced_branch() {
        let text = "    jr nz,.b\n    ld a,b\n    jr .over\n.b:\n    ld a,b\n    ld c,d\n.over:\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 7));
        let action = stabilize_action(&actions).expect("expected the stabilize action");
        assert_eq!(action.title, "Balance branch timings in selection");
        let edit = action.edit.as_ref().expect("expected an edit");
        let text_edits = edit.changes.as_ref().unwrap().get(&d.uri).unwrap();
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "    waitnops 1\n");
        assert_eq!(text_edits[0].range.start.line, 6);
    }

    /// The routine around the branch is itself a loop (the `jp .main` back-edge
    /// and a `djnz`), which used to make the whole selection ineligible. The
    /// `jr nz` diamond must still be balanced. Its taken arm is empty (it jumps
    /// straight to `.save_ptr`) and costs 3 NOPs; the fall-through costs 2 +
    /// `ld hl,nn` (3) + the guard `jr` (3) = 8, so the pad block holds 5.
    #[test]
    fn the_action_still_appears_for_a_branch_inside_a_loop() {
        let text = ".main\n\tld b,8\n.inner\n\tdjnz .inner\n\tld de, color_table_end\n\tor a : sbc hl, de : add hl, de ; Z if hl == color_table_end\n\tjr nz, .save_ptr\n\tld hl, color_table\n.save_ptr\n\tld (.ptr), hl\n\tjp .main\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 11));
        let action = stabilize_action(&actions).expect("expected the stabilize action");
        let edit = action.edit.as_ref().expect("expected an edit");
        let text_edits = edit.changes.as_ref().unwrap().get(&d.uri).unwrap();
        assert_eq!(text_edits.len(), 2, "{text_edits:?}");
        assert_eq!(text_edits[0].new_text, "jr nz,.__BASM__stabilize_pad_1");
        assert_eq!(
            text_edits[1].new_text,
            "    jr .save_ptr\n.__BASM__stabilize_pad_1\n    waitnops 5\n"
        );
        assert_eq!(text_edits[1].range.start.line, 8);
    }

    /// Applies `edits` (all expressed against the *original* text) and returns
    /// the rewritten lines.
    fn apply_edits(text: &str, edits: &[TextEdit]) -> Vec<String> {
        let mut sorted: Vec<&TextEdit> = edits.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse((e.range.start.line, e.range.start.character)));
        let mut out = text.to_string();
        let offset = |t: &str, p: Position| -> usize {
            let mut off = 0;
            for (i, l) in t.split_inclusive('\n').enumerate() {
                if i as u32 == p.line {
                    return off + p.character as usize;
                }
                off += l.len();
            }
            off
        };
        for e in sorted {
            let (a, b) = (offset(&out, e.range.start), offset(&out, e.range.end));
            out.replace_range(a..b, &e.new_text);
        }
        out.lines().map(str::to_string).collect()
    }

    /// Runs `lines` from the top to the end, forcing the one conditional jump
    /// to be taken or not, and returns the total cost in NOPs - computed from
    /// the timing table, independently of the balancing code. `waitnops N`
    /// is N NOPs.
    fn run_path(lines: &[String], condition_true: bool) -> u32 {
        use cpclib_z80flow::InstructionCost;
        let label_line = |name: &str| -> usize {
            // `global.local` -> the local label as written on its own line
            let name = name.find('.').map_or(name, |i| &name[i..]);
            lines
                .iter()
                .position(|l| l.split(';').next().unwrap().trim() == name.trim())
                .unwrap_or_else(|| panic!("no label {name} in {lines:?}"))
        };
        let (mut pc, mut total) = (0usize, 0u32);
        while pc < lines.len() {
            let code = lines[pc].split(';').next().unwrap().trim().to_string();
            pc += 1;
            let is_label = !lines[pc - 1].starts_with(char::is_whitespace);
            if code.is_empty() || is_label || code.ends_with(':') {
                continue;
            }
            // `a : b : c` chains
            for ins in code.split(" : ") {
                let ins = ins.trim();
                if let Some(n) = ins.strip_prefix("waitnops ") {
                    total += n.trim().parse::<u32>().unwrap();
                    continue;
                }
                let lower = ins.to_lowercase();
                if lower.starts_with("jr ") || lower.starts_with("jp ") {
                    let ops: Vec<&str> = ins[3..].split(',').map(str::trim).collect();
                    let (cond, target) = if ops.len() == 2 { (true, ops[1]) } else { (false, ops[0]) };
                    match crate::basm::timing::nops_of(ins) {
                        InstructionCost::Conditional { taken, not_taken } if cond => {
                            total += if condition_true { taken } else { not_taken };
                            if condition_true {
                                pc = label_line(target) + 1;
                            }
                        },
                        InstructionCost::Fixed(n) => {
                            total += n;
                            pc = label_line(target) + 1;
                        },
                        other => panic!("unexpected cost {other:?} for {ins}"),
                    }
                    break;
                }
                match crate::basm::timing::nops_of(ins) {
                    InstructionCost::Fixed(n) => total += n,
                    other => panic!("cannot cost `{ins}`: {other:?}"),
                }
            }
        }
        total
    }

    fn stabilized_lines(text: &str) -> Vec<String> {
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let n = text.lines().count() as u32;
        let actions = analyzer.code_actions(&d, full_line_range(0, n));
        let action = stabilize_action(&actions).expect("expected the stabilize action");
        let edit = action.edit.as_ref().expect("expected an edit");
        apply_edits(text, edit.changes.as_ref().unwrap().get(&d.uri).unwrap())
    }

    /// The user's real routine: `jr nz` straight to the merge label, so the
    /// taken arm is empty and cheaper. Padding must end up on the *taken*
    /// path only - the first version of this feature put it before
    /// `.save_ptr`, which the fall-through path also runs.
    #[test]
    fn an_empty_taken_arm_is_padded_on_the_taken_path_only() {
        let text = "start\n\tor a : sbc hl, bc : add hl, bc\n\tjr nz, .save_ptr\n\tld hl, color_table\n.save_ptr\n\tld (.ptr), hl\n";
        let before: Vec<String> = text.lines().map(str::to_string).collect();
        assert_ne!(
            run_path(&before, true),
            run_path(&before, false),
            "the input must start unbalanced"
        );
        let after = stabilized_lines(text);
        assert_eq!(
            run_path(&after, true),
            run_path(&after, false),
            "both paths must now cost the same: {after:#?}"
        );
    }

    /// Same, when the fall-through arm already ends in an unconditional jump
    /// to the merge point: no extra guard jump is needed (or wanted).
    #[test]
    fn an_empty_taken_arm_whose_other_arm_jumps_needs_no_guard() {
        let text = "start\n\tjr nz, .merge\n\tld a, b\n\tld c, d\n\tjr .merge\n.merge\n\tld e, h\n";
        let before: Vec<String> = text.lines().map(str::to_string).collect();
        assert_ne!(run_path(&before, true), run_path(&before, false));
        let after = stabilized_lines(text);
        assert_eq!(run_path(&after, true), run_path(&after, false), "{after:#?}");
    }

    /// Every shape the balancer handles, checked the same independent way:
    /// after the edits, forcing the conditional either way costs the same.
    #[test]
    fn stabilized_code_costs_the_same_on_both_paths_for_every_supported_shape() {
        let shapes = [
            // classic diamond, expensive taken arm
            "start\n\tjr nz,.b\n\tld a,b\n\tjr .over\n.b\n\tld a,b\n\tld c,d\n.over\n\tld e,h\n",
            // expensive fall-through arm, taken arm non-empty
            "start\n\tjr nz,.b\n\tld a,b\n\tld c,d\n\tld e,h\n\tjr .over\n.b\n\tld a,b\n.over\n\tld e,h\n",
            // empty taken arm, expensive fall-through (user's shape)
            "start\n\tjr z,.m\n\tld hl,1\n\tld a,b\n.m\n\tld e,h\n",
            // empty taken arm, fall-through is a single `ld`
            "start\n\tjr z,.m\n\tld a,b\n.m\n\tld e,h\n",
            // empty fall-through arm, expensive taken arm
            "start\n\tjr z,.b\n\tjr .m\n.b\n\tld a,b\n\tld c,d\n\tld e,h\n.m\n\tld e,h\n",
        ];
        for text in shapes {
            let before: Vec<String> = text.lines().map(str::to_string).collect();
            let (t, n) = (run_path(&before, true), run_path(&before, false));
            if t == n {
                continue;
            }
            let after = stabilized_lines(text);
            assert_eq!(
                run_path(&after, true),
                run_path(&after, false),
                "was {t} vs {n}; now: {after:#?}"
            );
        }
    }

    #[test]
    fn no_action_for_an_already_balanced_branch() {
        let text = "    jr nz,.b\n    nop\n    nop\n    jr .over\n.b:\n    nop\n    nop\n    nop\n    nop\n.over:\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 10));
        assert!(stabilize_action(&actions).is_none());
    }

    #[test]
    fn no_action_when_the_selection_contains_no_branch() {
        let d = doc("    ld a,b\n    nop\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 2));
        assert!(stabilize_action(&actions).is_none());
    }

    #[test]
    fn no_action_when_the_selection_contains_an_unsupported_construct() {
        // A DJNZ loop - unsupported (see `stabilize.rs`), must not crash or
        // offer an action, just silently decline like the loop-rejection
        // and mnemonic-rejection cases it wraps.
        let d = doc(".loop:\n    nop\n    djnz .loop\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 3));
        assert!(stabilize_action(&actions).is_none());
    }

    #[test]
    fn no_action_without_a_real_selection() {
        let d = doc("    jr nz,.b\n    nop\n.b:\n    nop\n    nop\n");
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
        let actions = analyzer.code_actions(&d, collapsed);
        assert!(stabilize_action(&actions).is_none());
    }

    /// A conditional RET's early-exit arm needing padding produces two
    /// edits (a range replace for the rewritten jump, plus one appended
    /// tail block), not the single zero-width insert the plain-padding
    /// path produces - exercises that richer shape end to end through
    /// `code_actions`, not just `stabilize_lines` directly.
    #[test]
    fn the_action_replaces_and_appends_for_a_conditional_ret_early_exit_arm() {
        let text = "bc26_hl\n    ld a,h\n    add 8\n    ld h,a\n    ret nc\n    ld bc,0xc000 + 96\n    add hl,bc\n    ret\n";
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 8));
        let action = stabilize_action(&actions).expect("expected the stabilize action");
        let edit = action.edit.as_ref().expect("expected an edit");
        let text_edits = edit.changes.as_ref().unwrap().get(&d.uri).unwrap();
        assert_eq!(text_edits.len(), 2, "{text_edits:?}");

        let replace = text_edits
            .iter()
            .find(|e| e.new_text.starts_with("jr "))
            .expect("expected the jump-rewrite edit");
        assert_eq!(replace.new_text, "jr nc,bc26_hl.__BASM__stabilize_pad_1");
        assert_eq!(replace.range.start, Position::new(4, 4));
        assert_eq!(replace.range.end, Position::new(4, 10));

        let append = text_edits
            .iter()
            .find(|e| e.new_text.starts_with(".__BASM__"))
            .expect("expected the appended tail block edit");
        assert_eq!(append.range.start, Position::new(8, 0));
        assert_eq!(append.range.start, append.range.end);
        // See `stabilize.rs`'s own hand-verified derivation of this count
        // (5, not the naive 7 - correcting for the RET-cc-vs-JR-cc timing
        // difference and the newly appended `ret`'s own cost).
        assert_eq!(
            append.new_text,
            ".__BASM__stabilize_pad_1\n    waitnops 5\n    ret\n"
        );
    }

    /// Same shape as above, but the document has no trailing newline after
    /// the selection's own last line (a real, common case - many editors
    /// don't force one). `str::lines()` produces the identical line list
    /// either way, so the selection/analysis side doesn't notice the
    /// difference - but appending at `(after_line + 1, character: 0)`
    /// blindly assumes a real newline already separates that position from
    /// the selection's last line, which isn't true here: the fix must
    /// anchor to the end of that last line's own content and supply its
    /// own leading newline instead, or the appended block glues onto the
    /// end of the last `ret` (`ret.__BASM__stabilize_pad_1`) instead of
    /// starting on its own line.
    #[test]
    fn the_appended_tail_block_gets_its_own_line_even_without_a_trailing_newline() {
        let text = "bc26_hl\n    ld a,h\n    add 8\n    ld h,a\n    ret nc\n    ld bc,0xc000 + 96\n    add hl,bc\n    ret";
        assert!(!text.ends_with('\n'));
        let d = doc(text);
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, full_line_range(0, 8));
        let action = stabilize_action(&actions).expect("expected the stabilize action");
        let edit = action.edit.as_ref().expect("expected an edit");
        let text_edits = edit.changes.as_ref().unwrap().get(&d.uri).unwrap();

        let append = text_edits
            .iter()
            .find(|e| e.new_text.contains("waitnops"))
            .expect("expected the appended tail block edit");
        assert_eq!(append.range.start, Position::new(7, 7));
        assert_eq!(append.range.start, append.range.end);
        assert_eq!(
            append.new_text,
            "\n.__BASM__stabilize_pad_1\n    waitnops 5\n    ret\n"
        );
    }
}
