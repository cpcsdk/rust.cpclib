//! Cycle (NOP) counting over a line range — shared core for both the
//! "cycle count for selection" code action (`command.rs`) and the VS Code
//! status-bar live display (`cpclib.cycleCountForSelection`, `backend.rs`).
//! Reuses the exact per-instruction timing lookup already backing
//! instruction hover (`timing::find_timings`), summing across every
//! recognized instruction in `[start_line, end_line]`.
//!
//! Deliberately text-based (not listing/token-based), matching this
//! feature area's established "must still work on an unparseable document"
//! philosophy (see `timing::format_hover`'s own text-only fallback path) -
//! reuses `timing::classify_line`, which already tells a real Z80
//! instruction apart from a directive, a label/blank line, or unrecognized
//! text (most likely a macro invocation).

use super::timing::{LineSegment, classify_line, find_timings};

/// Total NOP-count summary for a selected line range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct SelectionCycleCount {
    /// Sum of each instruction's cheaper cost (branch not taken, or the
    /// instruction's only cost if unconditional).
    pub min_nops: u32,
    /// Sum of each instruction's costlier cost (branch taken); equals
    /// `min_nops` when nothing in range is conditional.
    pub max_nops: u32,
    pub instruction_count: u32,
    /// Non-blank, non-directive content that didn't match any known Z80
    /// mnemonic (most likely a macro invocation) - the total is a lower
    /// bound when this is nonzero.
    pub unrecognized_count: u32
}

impl SelectionCycleCount {
    pub fn is_conditional(&self) -> bool {
        self.min_nops != self.max_nops
    }

    pub fn is_empty(&self) -> bool {
        self.instruction_count == 0 && self.unrecognized_count == 0
    }
}

/// Sum NOPs across every recognized instruction on lines `[start_line, end_line]`
/// (inclusive, 0-based) of `lines`. Out-of-range indices are simply skipped.
pub(super) fn count_cycles_in_lines(
    lines: &[&str],
    start_line: usize,
    end_line: usize
) -> SelectionCycleCount {
    let mut summary = SelectionCycleCount::default();
    for line in lines.iter().take(end_line + 1).skip(start_line) {
        for segment in classify_line(line) {
            match segment {
                LineSegment::Instruction(text) => {
                    let Some(entry) = find_timings(&text).into_iter().next()
                    else {
                        summary.unrecognized_count += 1;
                        continue;
                    };
                    let alt = entry.nops_alt.unwrap_or(entry.nops);
                    summary.min_nops += entry.nops.min(alt) as u32;
                    summary.max_nops += entry.nops.max(alt) as u32;
                    summary.instruction_count += 1;
                },
                LineSegment::Unrecognized => summary.unrecognized_count += 1,
                LineSegment::Directive | LineSegment::Blank => {}
            }
        }
    }
    summary
}

/// Human-readable one-line summary for the code action's title - NOPs only,
/// never T-states (see the module doc comment on `timing.rs`'s own
/// established "NOPs, not T-states" convention).
pub(super) fn format_title(summary: &SelectionCycleCount) -> String {
    let mut title = if summary.is_conditional() {
        format!(
            "Cycle count: {}-{} NOPs (branch not taken/taken)",
            summary.min_nops, summary.max_nops
        )
    }
    else {
        format!("Cycle count: {} NOPs", summary.min_nops)
    };
    if summary.unrecognized_count > 0 {
        title.push_str(&format!(
            ", {} line{} not counted",
            summary.unrecognized_count,
            if summary.unrecognized_count == 1 {
                ""
            }
            else {
                "s"
            }
        ));
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn an_unconditional_sequence_sums_to_a_single_fixed_total() {
        let l = lines("    ld a, b\n    ld c, d\n    nop\n");
        let summary = count_cycles_in_lines(&l, 0, 2);
        assert!(!summary.is_conditional(), "{summary:?}");
        assert_eq!(summary.instruction_count, 3);
        assert_eq!(summary.unrecognized_count, 0);
        // ld r,r' = 1 NOP, ld r,r' = 1 NOP, nop = 1 NOP
        assert_eq!(summary.min_nops, 3);
        assert_eq!(summary.max_nops, 3);
    }

    #[test]
    fn a_conditional_jump_produces_a_min_max_range() {
        let l = lines("loop: djnz loop\n");
        let summary = count_cycles_in_lines(&l, 0, 0);
        assert!(summary.is_conditional(), "{summary:?}");
        assert_eq!(summary.min_nops, 3);
        assert_eq!(summary.max_nops, 4);
        assert_eq!(summary.instruction_count, 1);
    }

    #[test]
    fn a_directive_line_contributes_zero_and_is_not_flagged_unrecognized() {
        let l = lines("    db 1,2,3\n    org 0x4000\n");
        let summary = count_cycles_in_lines(&l, 0, 1);
        assert_eq!(summary.instruction_count, 0);
        assert_eq!(summary.unrecognized_count, 0);
        assert_eq!(summary.min_nops, 0);
        assert!(summary.is_empty());
    }

    #[test]
    fn an_unrecognized_identifier_increments_the_unrecognized_count_and_is_excluded() {
        let l = lines("    call MY_UNDEFINED_MACRO_LOOKING_THING_XYZ\n");
        // "call" itself IS a real instruction (unconditional CALL nn) - use
        // something whose *leading word* isn't a known mnemonic/directive
        // to exercise the truly-unrecognized path.
        let l2 = lines("    frobnicate a, b\n");
        let summary = count_cycles_in_lines(&l2, 0, 0);
        assert_eq!(summary.instruction_count, 0);
        assert_eq!(summary.unrecognized_count, 1);
        assert_eq!(summary.min_nops, 0);
        assert!(!summary.is_empty());
        // Sanity: "call" alone is a real instruction, not unrecognized.
        let summary2 = count_cycles_in_lines(&l, 0, 0);
        assert_eq!(summary2.instruction_count, 1);
        assert_eq!(summary2.unrecognized_count, 0);
    }

    #[test]
    fn multiple_colon_separated_instructions_on_one_line_are_all_counted() {
        let l = lines("    ld a,b : ld c,d : nop\n");
        let summary = count_cycles_in_lines(&l, 0, 0);
        assert_eq!(summary.instruction_count, 3);
        assert_eq!(summary.min_nops, 3);
    }

    #[test]
    fn a_label_only_line_and_a_blank_line_are_ignored() {
        let l = lines("loop:\n\n    nop\n");
        let summary = count_cycles_in_lines(&l, 0, 2);
        assert_eq!(summary.instruction_count, 1);
        assert_eq!(summary.unrecognized_count, 0);
        assert_eq!(summary.min_nops, 1);
    }

    #[test]
    fn format_title_shows_a_range_when_conditional() {
        let summary = SelectionCycleCount {
            min_nops: 8,
            max_nops: 12,
            instruction_count: 4,
            unrecognized_count: 0
        };
        assert_eq!(
            format_title(&summary),
            "Cycle count: 8-12 NOPs (branch not taken/taken)"
        );
    }

    #[test]
    fn format_title_notes_unrecognized_lines() {
        let summary = SelectionCycleCount {
            min_nops: 4,
            max_nops: 4,
            instruction_count: 2,
            unrecognized_count: 1
        };
        assert_eq!(
            format_title(&summary),
            "Cycle count: 4 NOPs, 1 line not counted"
        );
    }
}
