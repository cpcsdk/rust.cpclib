//! Min/max cycle (NOP) count over a selection — shared core for both the
//! "cycle count for selection" code action (`command.rs`) and the VS Code
//! status-bar live display (`cpclib.cycleCountForSelection`, `backend.rs`).
//!
//! Genuinely control-flow-aware: filters the document's already-parsed,
//! cached `LocatedListing` down to the selection (via `token::
//! tokens_in_range`/`tokens_in_lines`, borrowed tokens, never re-parsed)
//! and delegates to `cpclib_asm::cost_range::cost_range`, which builds a
//! real CFG and reports the *distinct* min/max path costs - not a naive
//! per-line sum. That distinction matters: a selection containing a
//! `JR`-and-merge diamond has *both* arms' instructions in its text, but
//! only one ever executes per real run - summing every line regardless
//! (the pre-refactor behavior here) silently double-counts both arms
//! together, a real bug this rewrite fixes (found by the user checking a
//! freshly-balanced selection and getting a visibly wrong total).
//!
//! Requires a successful parse - unlike this module's own previous
//! text-based version, there is no "must still work on unparseable text"
//! fallback here anymore (an explicit, deliberate simplification: this
//! LSP's own `hover.rs`/`call_hierarchy.rs`/`autocomplete.rs` etc. already
//! all just do nothing on a parse error via `self.parse_document(document)
//! .ok()`, so this module joining that established convention isn't a
//! special exception).

use cpclib_asm::parser::obtained::{LocatedListing, LocatedToken, LocatedTokenInner};
use cpclib_tokens::{ExprElement, ListingElement};
use cpclib_z80flow::cost_range::{self};
use cpclib_z80flow::{CostModel, InstructionCost};
use tower_lsp::lsp_types::{Position, Range};

use super::timing::nops_of;
use super::token::tokens_in_range;

/// Total NOP-count summary for a selected line range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct SelectionCycleCount {
    /// The cheapest real runtime path's total cost.
    pub min_nops: u32,
    /// The costliest real runtime path's total cost; equals `min_nops`
    /// when nothing in range is conditional. Meaningless as a real upper
    /// bound when `max_unbounded` is true (see below) - a partial sum, not
    /// the actual worst case.
    pub max_nops: u32,
    /// True when the range contains a loop (a `DJNZ`, or a backward
    /// `JR`/`JP`) whose real iteration count isn't statically known here -
    /// the real worst case is unbounded, not the partial sum `max_nops`
    /// would otherwise settle on. Sticky: propagates through anything that
    /// contains such a loop anywhere in its own path.
    pub max_unbounded: bool,
    pub instruction_count: u32,
    /// Recognized-as-an-instruction tokens whose cost source didn't know a
    /// timing for them (or, unlike balancing, any *other* non-executing
    /// token this feature doesn't specifically recognize as zero-cost,
    /// e.g. a macro invocation) - the total is a lower bound when this is
    /// nonzero.
    pub unrecognized_count: u32,
    /// A `CALL` in range reaches a routine defined outside the analysed
    /// tokens, so only the call instruction itself could be priced.
    ///
    /// The counts are for a selection the user can see; this one is about
    /// what they *cannot*: a routine in another file, or above the selection
    /// they made. Worth saying out loud, because the number looks complete
    /// either way.
    pub incomplete: bool
}

impl SelectionCycleCount {
    pub fn is_conditional(&self) -> bool {
        self.max_unbounded || self.min_nops != self.max_nops
    }

    pub fn is_empty(&self) -> bool {
        self.instruction_count == 0 && self.unrecognized_count == 0
    }
}

/// `WAITNOPS <expr>` - the exact directive `stabilize.rs`'s own Quick Fix
/// generates for padding - is the one directive `is_directive()` reports
/// that genuinely emits runtime bytes with a real, non-zero cost, unlike a
/// true bookkeeping directive (`DB`/`ORG`/`EQU`/...), which `is_directive()`
/// doesn't distinguish from it at all (a direct probe against real parsed
/// tokens confirmed both report `is_directive() == true` identically).
/// `ListingElement` has no generic accessor for a directive's own
/// argument, so this matches `LocatedToken`'s own `Deref<Target =
/// LocatedTokenInner>` directly to reach `LocatedTokenInner::WaitNops` -
/// *not* via `ToSimpleToken::as_simple_token()` (the more obvious route),
/// which turns out to have its own pre-existing `unimplemented!()` for
/// this exact variant (`cpclib-asm/src/parser/obtained.rs:1959`, a real
/// gap in this codebase, not introduced here - found by hitting it).
/// Reads the expression only when it's a bare integer literal
/// (`ExprElement::is_value`), which is *always* the case for
/// `stabilize.rs`'s own generated output (`waitnops {nop_count}`, always a
/// plain number). A hand-written `waitnops duration(...) + ...`-style
/// expression (real, symbolic, and possible - see the user's own examples
/// motivating this whole feature) would need genuine expression
/// evaluation (register/symbol state, `duration()` itself needing
/// instruction-cost lookups) - a materially bigger undertaking, not
/// attempted here.
///
/// `Some(Fixed(n))` for a bare integer literal argument; `Some(Unknown)`
/// for anything else (a real, symbolic expression this module doesn't
/// evaluate); `None` when `token` isn't `WAITNOPS` at all (the caller
/// falls back to its own generic directive handling in that case).
fn waitnops_cost(token: &LocatedToken) -> Option<InstructionCost> {
    let LocatedTokenInner::WaitNops(expr) = &**token
    else {
        return None;
    };
    Some(if expr.is_value() {
        InstructionCost::Fixed(expr.value() as u32)
    }
    else {
        InstructionCost::Unknown
    })
}

/// This instruction's cost, from the existing `timing.rs` NOPs table.
/// Deliberately *stricter* than `stabilize.rs`'s own `cost_from_timing`:
/// that closure treats any non-instruction token (including a macro
/// invocation, which still parses as some token even though out of
/// `branch_balance`'s declared scope) as harmless `Fixed(0)` - correct for
/// balancing (an irrelevant shared zero cancels out of a delta) but wrong
/// here, where it would silently *undercount* a macro call with no
/// warning at all. `Fixed(0)` only for tokens genuinely known to be
/// non-executing (a comment, or a bookkeeping directive) - everything
/// else with no recognized timing entry is `Unknown`, incrementing
/// `unrecognized_count` instead of vanishing silently.
struct StrictTimingCosts;

impl CostModel<LocatedToken> for StrictTimingCosts {
    fn cost(&self, token: &LocatedToken) -> InstructionCost {
        if token.mnemonic().is_none() {
            if token.is_directive()
                && let Some(cost) = waitnops_cost(token)
            {
                return cost;
            }
            return if token.is_comment() || token.is_directive() {
                InstructionCost::Fixed(0)
            }
            else {
                InstructionCost::Unknown
            };
        }
        nops_of(&token.to_string())
    }
}

/// The min/max cost summary for every token inside `range` of the
/// already-parsed `listing` (character-accurate - see `token::
/// tokens_in_range`). Empty (`SelectionCycleCount::default()`) when
/// nothing recognizable is in range, or on the rare genuine parse-shape
/// anomaly `cost_range` itself can't interpret (see its own doc comment) -
/// callers treat an empty summary as "nothing to show", matching this
/// module's own pre-existing convention.
pub(super) fn count_cycles_in_selection(
    listing: &LocatedListing,
    range: Range
) -> SelectionCycleCount {
    let tokens = tokens_in_range(listing.iter(), range);
    let Ok(result) = cost_range::cost_range(&tokens, StrictTimingCosts)
    else {
        return SelectionCycleCount::default();
    };
    SelectionCycleCount {
        min_nops: result.min,
        max_nops: result.max,
        incomplete: result.incomplete,
        max_unbounded: result.unbounded,
        instruction_count: result.instruction_count,
        unrecognized_count: result.unrecognized_count
    }
}

/// Whole-line convenience wrapper over [`count_cycles_in_selection`] -
/// covers the entirety of both `start_line` and `end_line` (inclusive,
/// 0-based), ignoring any character position within them.
pub(super) fn count_cycles_in_lines(
    listing: &LocatedListing,
    start_line: usize,
    end_line: usize
) -> SelectionCycleCount {
    count_cycles_in_selection(
        listing,
        Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        }
    )
}

/// Human-readable one-line summary for the code action's title - NOPs only,
/// never T-states (this module's own established "NOPs, not T-states"
/// convention, matching `timing.rs`'s own hover text).
pub(super) fn format_title(summary: &SelectionCycleCount) -> String {
    let mut title = if summary.max_unbounded {
        format!(
            "Cycle count: {}-? NOPs (unbounded: a loop's iteration count isn't statically known)",
            summary.min_nops
        )
    }
    else if summary.is_conditional() {
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
            ", {} instruction{} not counted",
            summary.unrecognized_count,
            if summary.unrecognized_count == 1 {
                ""
            }
            else {
                "s"
            }
        ));
    }
    if summary.incomplete {
        // The cost of a `call` now includes the routine it calls - so when
        // one of them could not be found, the total is short by an unknown
        // amount and the user should not read it as final.
        title.push_str(", a called routine is outside the selection");
    }
    title
}

#[cfg(test)]
mod tests {
    use cpclib_asm::parser::parse_z80_str;

    use super::*;

    fn summary(text: &str) -> SelectionCycleCount {
        let listing = parse_z80_str(text).unwrap();
        let end_line = text.lines().count().saturating_sub(1);
        count_cycles_in_lines(&listing, 0, end_line)
    }

    /// The end-to-end check that call-following is driven by the *real*
    /// timing table and not by any number written into the code.
    ///
    /// `data/timings.txt` says `call nn` is 5 and `ret` is 3, so `call go` +
    /// `ret` + a body of `nop` + `ret` is 5 + (1 + 3) + 3 = 12. The number
    /// that matters is that it is not 8 - which is what the old behaviour
    /// gave, charging the `call` and ignoring what it called.
    #[test]
    fn a_call_is_priced_with_its_callee_using_the_real_timing_table() {
        let s = summary("    call go\n    ret\ngo\n    nop\n    ret\n");
        assert_eq!(s.min_nops, 12, "{s:?}");
        assert_eq!(s.max_nops, 12, "{s:?}");
        assert!(!s.incomplete, "{s:?}");
    }

    /// `call ccc,nn` is `5 or 3 if /ccc/ not met` in `data/timings.txt`, and
    /// that `nops_alt` is what makes a conditional call bound the two ends
    /// differently: 3 without the callee, 5 + the body with it.
    #[test]
    fn a_conditional_call_reads_both_of_its_timings_from_the_table() {
        let s = summary("    call nz,go\n    ret\ngo\n    nop\n    ret\n");
        assert_eq!(s.min_nops, 3 + 3, "not taken, then ret: {s:?}");
        assert_eq!(s.max_nops, 5 + 4 + 3, "taken, callee body, then ret: {s:?}");
        assert!(s.is_conditional(), "{s:?}");
    }

    /// A routine that is not in view cannot be priced, and the summary says
    /// so rather than quietly reporting a total that is short.
    #[test]
    fn a_call_to_a_routine_outside_the_selection_is_reported_incomplete() {
        let s = summary("    call elsewhere\n    ret\n");
        assert_eq!(s.min_nops, 5 + 3, "{s:?}");
        assert!(s.incomplete, "{s:?}");
        assert!(
            format_title(&s).contains("outside the selection"),
            "{}",
            format_title(&s)
        );
    }

    /// `ld hl, de` is not a Z80 opcode - basm assembles it to `ld h, d` /
    /// `ld l, e`, two real instructions at 1 NOP each. The timing table is
    /// keyed by instruction text and has no entry for the written form, so
    /// before the cost model could ask about an expansion this contributed
    /// **zero** and merely bumped `unrecognized_count`. The corpus has 29 of
    /// these across 15 files.
    #[test]
    fn a_fake_instruction_costs_what_it_assembles_to() {
        let s = summary("    ld hl, de\n    nop\n");
        assert_eq!(s.min_nops, 3, "ld h,d + ld l,e + nop: {s:?}");
        assert_eq!(s.max_nops, 3, "{s:?}");
        assert_eq!(
            s.unrecognized_count, 0,
            "the expansion is fully priced, so nothing is unrecognized: {s:?}"
        );
    }

    /// The control: a real 16-bit load is a single opcode with its own entry,
    /// so the test above is not just measuring "any ld costs 2".
    #[test]
    fn a_real_sixteen_bit_load_is_still_one_instruction() {
        let s = summary("    ld hl, 0x4000\n    nop\n");
        assert_eq!(s.min_nops, 4, "ld hl,nn is 3 NOPs, plus nop: {s:?}");
        assert_eq!(s.unrecognized_count, 0, "{s:?}");
    }

    /// `jq` is basm's "assembler picks JR or JP" form, absent from the timing
    /// table for the same reason a fake instruction is. Unconditionally both
    /// candidates cost 3, so the answer does not depend on which one basm
    /// picks - and the cost model asks for both rather than assuming that.
    #[test]
    fn an_unconditional_jq_costs_what_both_candidates_agree_on() {
        let s = summary("    jq elsewhere\n");
        assert_eq!(s.min_nops, 3, "jr and jp are both 3 NOPs: {s:?}");
        assert_eq!(s.unrecognized_count, 0, "{s:?}");
    }

    /// A *conditional* `jq` genuinely differs between the two candidates
    /// (`jr cc` is "3 or 2", `jp cc` is always 3), so it stays unknown rather
    /// than picking one. Which it is depends on a distance only a real
    /// assemble knows.
    #[test]
    fn a_conditional_jq_stays_unknown_because_the_candidates_disagree() {
        let s = summary("    jq nz, elsewhere\n");
        assert_eq!(
            s.unrecognized_count, 1,
            "the two candidates disagree, so this must not be guessed: {s:?}"
        );
    }

    #[test]
    fn an_unconditional_sequence_sums_to_a_single_fixed_total() {
        let s = summary("    ld a, b\n    ld c, d\n    nop\n");
        assert!(!s.is_conditional(), "{s:?}");
        assert_eq!(s.instruction_count, 3);
        assert_eq!(s.unrecognized_count, 0);
        // ld r,r' = 1 NOP, ld r,r' = 1 NOP, nop = 1 NOP
        assert_eq!(s.min_nops, 3);
        assert_eq!(s.max_nops, 3);
    }

    #[test]
    fn a_conditional_jump_produces_a_min_max_range() {
        let s = summary("    jr nz,.x\n.x\n    nop\n");
        assert!(s.is_conditional(), "{s:?}");
        // jr nz taken (3) vs not-taken (2) + nop (1) = 3 -> already equal
        // here; use a genuinely asymmetric shape instead.
        let s2 = summary("    jr nz,.x\n    nop\n    nop\n.x\n");
        assert!(s2.is_conditional(), "{s2:?}");
        assert_eq!(s2.min_nops, 3, "{s2:?}"); // taken: jr(3)
        assert_eq!(s2.max_nops, 4, "{s2:?}"); // not-taken: jr(2)+nop+nop
    }

    /// A `DJNZ` loop reports a real, meaningful `unbounded` max instead of
    /// a wrong flat number - the loop's iteration count isn't statically
    /// known in general.
    #[test]
    fn a_djnz_loop_reports_an_unbounded_max_not_a_flat_wrong_number() {
        let s = summary(".loop\n    nop\n    djnz .loop\n");
        assert!(s.max_unbounded, "{s:?}");
        assert!(s.is_conditional(), "{s:?}");
    }

    #[test]
    fn a_directive_line_contributes_zero_and_is_not_flagged_unrecognized() {
        let s = summary("    db 1,2,3\n    org 0x4000\n");
        assert_eq!(s.instruction_count, 0);
        assert_eq!(s.unrecognized_count, 0);
        assert_eq!(s.min_nops, 0);
        assert!(s.is_empty());
    }

    #[test]
    fn a_label_only_line_and_a_blank_line_are_ignored() {
        let s = summary("loop:\n\n    nop\n");
        assert_eq!(s.instruction_count, 1);
        assert_eq!(s.unrecognized_count, 0);
        assert_eq!(s.min_nops, 1);
    }

    #[test]
    fn multiple_colon_separated_instructions_on_one_line_are_all_counted() {
        let s = summary("    ld a,b : ld c,d : nop\n");
        assert_eq!(s.instruction_count, 3);
        assert_eq!(s.min_nops, 3);
    }

    #[test]
    fn format_title_shows_a_range_when_conditional() {
        let s = SelectionCycleCount {
            min_nops: 8,
            max_nops: 12,
            max_unbounded: false,
            instruction_count: 4,
            unrecognized_count: 0,
            incomplete: false
        };
        assert_eq!(
            format_title(&s),
            "Cycle count: 8-12 NOPs (branch not taken/taken)"
        );
    }

    #[test]
    fn format_title_notes_unrecognized_instructions() {
        let s = SelectionCycleCount {
            min_nops: 4,
            max_nops: 4,
            max_unbounded: false,
            instruction_count: 2,
            unrecognized_count: 1,
            incomplete: false
        };
        assert_eq!(
            format_title(&s),
            "Cycle count: 4 NOPs, 1 instruction not counted"
        );
    }

    /// The exact motivating regression, shape-wise: a `JR`-and-merge
    /// diamond selection (what a freshly-balanced conditional produces)
    /// must report the real per-path cost, not the sum of both arms' text.
    /// Uses `nop`*5 rather than `waitnops 5` for the padding deliberately
    /// (see `waitnops_is_not_yet_recognized_a_real_remaining_gap` below for
    /// why) so this test isolates *only* the CFG-awareness fix. Hand
    /// verified: taken = prefix(4) + jr_taken(3) + nop*5(5) + ret(3) = 15;
    /// not-taken = prefix(4) + jr_not_taken(2) + ld bc,nn(3) + add hl,bc(3)
    /// + ret(3) = 15 - both paths equal, as balanced code should be. The
    /// pre-refactor naive summer would have reported both arms' text added
    /// together instead (a much larger, wrong number).
    #[test]
    fn a_balanced_diamond_reports_equal_paths_not_a_naive_sum() {
        let s = summary(
            "\
bc26_hl
    ld a,h
    add 8
    ld h,a
    jr nc,bc26_hl.pad
    ld bc,0xc000 + 96
    add hl,bc
    ret
.pad
    nop
    nop
    nop
    nop
    nop
    ret
"
        );
        assert!(!s.is_conditional(), "{s:?}");
        assert_eq!(s.min_nops, 15, "{s:?}");
        assert_eq!(s.max_nops, 15, "{s:?}");
        assert_eq!(s.unrecognized_count, 0, "{s:?}");
    }

    /// The exact motivating regression, verbatim: `WAITNOPS <expr>` - the
    /// exact directive `stabilize.rs`'s own Quick Fix generates for padding
    /// - isn't a real Z80 mnemonic, so it has no entry in
    /// `data/timings.txt`; `waitnops_cost` reads its literal argument
    /// directly instead (`stabilize.rs` only ever generates a bare integer
    /// literal, never a symbolic expression). Hand-verified identically to
    /// the `nop`*5 version above: both paths total 15.
    #[test]
    fn a_real_waitnops_literal_is_recognized_and_counted() {
        let s = summary(
            "\
bc26_hl
    ld a,h
    add 8
    ld h,a
    jr nc,bc26_hl.pad
    ld bc,0xc000 + 96
    add hl,bc
    ret
.pad
    waitnops 5
    ret
"
        );
        assert!(!s.is_conditional(), "{s:?}");
        assert_eq!(s.min_nops, 15, "{s:?}");
        assert_eq!(s.max_nops, 15, "{s:?}");
        assert_eq!(s.unrecognized_count, 0, "{s:?}");
    }

    /// A real, currently-open (and honestly reported, not silently
    /// papered over) boundary: `WAITNOPS` with a genuinely *symbolic*
    /// expression (arbitrary arithmetic, e.g. the user's own
    /// `duration(...) + duration(...) - 1` idiom) needs real expression
    /// evaluation (register/symbol state, `duration()` itself needing
    /// instruction-cost lookups) - a materially bigger undertaking than
    /// reading a bare literal, not attempted here.
    #[test]
    fn a_symbolic_waitnops_expression_is_still_flagged_unrecognized() {
        let s = summary("    waitnops n + 1\n");
        assert_eq!(s.unrecognized_count, 1, "{s:?}");
        assert_eq!(s.min_nops, 0, "{s:?}");
    }
}
