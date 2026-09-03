//! LSP adapter for `cpclib_asm::branch_balance`: filters the document's
//! already-parsed, cached `LocatedListing` down to the selection and back.
//! The actual CFG/post-dominator/balancing algorithm lives in `cpclib-asm`
//! (generic over `ListingElement`, so it isn't tied to the LSP) - this
//! module only does three things:
//! - filters the cached listing to the selection via `token::
//!   tokens_in_range`/`tokens_in_lines` (borrowed tokens, never
//!   re-parsed and never cloned - `LocatedToken::Clone` is an
//!   `unimplemented!()` stub in this codebase, so cloning was never an
//!   option anyway);
//! - supplies a cost function wrapping the already-vetted `timing.rs` NOPs
//!   table (`find_timings`) - `cpclib-asm` carries no Z80 timing data of its
//!   own, so this table stays the single source of truth for the LSP;
//! - renders the returned `StabilizeEdit`s into real source text: a plain
//!   `waitnops N` insertion, or - for a conditional `RET`'s early-exit arm,
//!   which has no in-selection code to pad into - a `JR cc,<label>`
//!   rewrite plus an appended padded tail block.
//!
//! Working on real tokens rather than raw text also means a branch sharing
//! its source line with other `:`-chained instructions (e.g.
//! `ld a,b : jr nz,.label`) is no longer a special case to guard against -
//! each `:`-segment already parses into its own separate token with its own
//! span, so the CFG builder sees the jump exactly like any other, no matter
//! how the line was written.
//!
//! ## Qualifying the synthesized label
//!
//! When a conditional `RET`'s early-exit arm needs padding, the new tail
//! block's own label is written bare (`.__BASM__stabilize_pad_N`) - a
//! plain (non-dot) label would call basm's `set_current_global_label` and
//! silently reset `.local` scoping for every subsequent dot-label in the
//! rest of the file. But the *reference* to it, inside the rewritten `JR
//! cc,<label>`, is written fully qualified
//! (`<owning_global>.__BASM__stabilize_pad_N`) rather than relying on
//! basm's own ambient "current global label" tracking at the reference
//! site - the same convention `qualify_local_symbol` already uses for
//! listing output (`cpclib-asm/src/assembler/listing_output/render/
//! shared.rs`). `owning_global` is found by scanning the tokens before the
//! `RET` for the nearest one whose `label_symbol()` doesn't start with
//! `.` - the same rule basm's own `set_current_global_label` uses to
//! decide whether a label updates the current global. This is a faithful
//! mirror of basm's real behavior *because* this whole feature already
//! deliberately excludes macros/repeats (decided at the very start of its
//! design), so a plain sequential per-selection scan can't diverge from
//! what basm's own multi-pass assembly would actually do. If no such label
//! exists in the selection, the rewrite is refused (the whole Quick Fix
//! reports no edits) rather than emit a reference that might not reliably
//! resolve.

use cpclib_asm::parser::obtained::{LocatedListing, LocatedToken};
use cpclib_tokens::ListingElement;
use cpclib_z80flow::branch_balance::{self, StabilizeEdit};
use cpclib_z80flow::{CostModel, InstructionCost};
use tower_lsp::lsp_types::{Position, Range};

use super::timing::{find_timings, nops_of, split_head};
use super::token::{token_lsp_range, tokens_in_range};

/// One point of text change needed to balance the selection.
pub(super) enum StabilizeTextEdit {
    /// Insert `nop_count` cost-units' worth of padding right before `line`
    /// (0-based, a real document line).
    InsertPadding { line: usize, nop_count: u32 },
    /// Replace `range` (a conditional `RET`'s own span) with `new_text`
    /// (`"JR cc,<qualified label>"`).
    ReplaceRetWithJump { range: Range, new_text: String },
    /// Append `text` (one or more padded tail blocks, already newline
    /// terminated) as a zero-width insert right at `at` - always the start
    /// of a real line (never mid-line, even for a precise/partial
    /// selection - see `stabilize_selection`'s own computation of this
    /// position). Every rewritten `RET` in one Quick Fix invocation shares
    /// a single append edit (rather than each getting its own zero-width
    /// insert at the same position), since several edits touching the
    /// identical position is ambiguous to apply.
    AppendTailBlocks { at: Position, text: String }
}

/// This instruction's cost, from the existing `timing.rs` NOPs table -
/// `Unknown` (aborting the whole pass, per `InstructionCost`'s own
/// fail-safe policy) for a real instruction `find_timings` doesn't
/// recognize, `Fixed(0)` for anything that isn't an instruction at all
/// (labels, comments, directives, ...), matching how those contributed
/// nothing to the old text-based version's own cost sum.
struct TimingCosts;

impl CostModel<LocatedToken> for TimingCosts {
    fn cost(&self, token: &LocatedToken) -> InstructionCost {
        if token.mnemonic().is_none() {
            return InstructionCost::Fixed(0);
        }
        nops_of(&token.to_string())
    }
}

/// `(taken, not_taken)` nops for a conditional instruction's real timing
/// entry (`None` for either side this table doesn't recognize).
fn conditional_cost(text: &str) -> Option<(u32, u32)> {
    let entry = find_timings(text).into_iter().next()?;
    Some((entry.nops as u32, entry.nops_alt? as u32))
}

/// `cpclib-asm`'s `RewriteConditionalRetAndPad.nop_count` is computed
/// against the *original* `RET cc` token - correct only if that
/// instruction stayed exactly as-is with zero further content on its
/// taken side, which was true right up until this rewrite exists. Once
/// rewritten, the taken side actually runs `JR cc` (not `RET cc` - a
/// different real cost, both taken and not-taken) followed by the padding
/// *and a brand-new trailing `RET`* that wasn't part of the original code
/// at all. Both effects need correcting for before the padding count is
/// right:
///
/// ```text
/// target:      jr_taken + N + new_ret_cost == not_taken_real
/// not_taken_real = raw_nop_count + ret_taken + (jr_not_taken - ret_not_taken)
/// (since cpclib-asm's own raw_nop_count = not_taken_real - ret_taken,
///  assuming the taken side had zero extra cost)
/// =>  N = raw_nop_count
///         + (ret_taken - jr_taken)
///         + (jr_not_taken - ret_not_taken)
///         - new_ret_cost
/// ```
///
/// `Err` (refusing the whole rewrite, not emitting a wrong count) when the
/// correction would go negative - meaning the *un*corrected numbers
/// `cpclib-asm` used to decide "the taken arm is cheaper" in the first
/// place no longer hold once the real `JR`+trailing-`RET` overhead is
/// accounted for; re-deciding which arm to pad from scratch with corrected
/// numbers isn't done here (a real, known limitation, not an oversight).
fn corrected_rewrite_nop_count(
    condition_text: &str,
    ret_text: &str,
    raw_nop_count: u32
) -> Result<u32, String> {
    let (ret_taken, ret_not_taken) =
        conditional_cost(ret_text).ok_or_else(|| format!("unknown timing for \"{ret_text}\""))?;
    let jr_text = format!("jr {condition_text},x");
    let (jr_taken, jr_not_taken) =
        conditional_cost(&jr_text).ok_or_else(|| format!("unknown timing for \"{jr_text}\""))?;
    let new_ret_cost = find_timings("ret")
        .first()
        .map(|e| e.nops as u32)
        .ok_or_else(|| "unknown timing for \"ret\"".to_string())?;

    let corrected = raw_nop_count as i64
        + (ret_taken as i64 - jr_taken as i64)
        + (jr_not_taken as i64 - ret_not_taken as i64)
        - new_ret_cost as i64;
    if corrected < 0 {
        return Err(
            "the rewritten JR's own timing overhead makes the early-exit arm no longer the \
             cheaper one - not supported"
                .to_string()
        );
    }
    Ok(corrected as u32)
}

/// The nearest label at or before `before_index` (an index into `tokens`)
/// whose name doesn't start with `.` - the same rule basm's own
/// `set_current_global_label` uses to decide whether a label updates the
/// current global (see this module's own doc comment for why a plain
/// per-selection scan is a faithful mirror of basm's real behavior here).
fn owning_global_label<'a>(tokens: &[&'a LocatedToken], before_index: usize) -> Option<&'a str> {
    tokens[..before_index]
        .iter()
        .rev()
        .find(|t| t.is_label() && !t.label_symbol().starts_with('.'))
        .map(|t| t.label_symbol())
}

/// Detects and balances every hand-written `JR`/`JP`/`RET` branch inside
/// `range` of the already-parsed `listing` (character-accurate - see
/// `token::tokens_in_range`). Empty when the selection is already
/// balanced. `Err` with a human-readable reason when the selection
/// contains something this v1 doesn't support (a loop, `DJNZ`/`CALL`, an
/// escaping jump target, a conditional `RET` needing a rewrite with no
/// qualifying global label in view, ...) - the caller is expected to offer
/// no action at all in that case, not surface the message as an error
/// popup (this isn't necessarily a mistake the user made, just a
/// selection shape this feature doesn't cover yet).
pub(super) fn stabilize_selection(
    listing: &LocatedListing,
    range: Range
) -> Result<Vec<StabilizeTextEdit>, String> {
    let tokens = tokens_in_range(listing.iter(), range);

    let raw_edits = branch_balance::balance_branches(&tokens, TimingCosts)?;

    let mut edits = Vec::with_capacity(raw_edits.len());
    let mut tail_blocks = String::new();
    let mut label_counter = 0u32;

    for edit in raw_edits {
        match edit {
            StabilizeEdit::InsertPadding {
                insert_before_index,
                nop_count
            } => {
                // An index at (or past) `tokens.len()` means "insert after
                // everything in the selection" (see `arm_padding_index`'s
                // own fallback) - there is no token there to read a span
                // from, so it maps to the selection's own end position.
                let line = if insert_before_index < tokens.len() {
                    token_lsp_range(tokens[insert_before_index]).start.line as usize
                }
                else {
                    range.end.line as usize
                };
                edits.push(StabilizeTextEdit::InsertPadding { line, nop_count });
            },
            StabilizeEdit::RewriteConditionalRetAndPad {
                ret_token_index,
                nop_count
            } => {
                let owning_global =
                    owning_global_label(&tokens, ret_token_index).ok_or_else(|| {
                        "a conditional RET's early-exit arm needs padding, but no enclosing \
                         global label was found in the selection to qualify the new local \
                         label - include the routine's own label in the selection"
                            .to_string()
                    })?;
                label_counter += 1;
                let bare_label = format!(".__BASM__stabilize_pad_{label_counter}");
                let qualified_label = format!("{owning_global}{bare_label}");

                let ret_token = tokens[ret_token_index];
                let ret_text = ret_token.to_string();
                let condition_text = split_head(&ret_text).1.to_string();
                let nop_count = corrected_rewrite_nop_count(&condition_text, &ret_text, nop_count)?;

                edits.push(StabilizeTextEdit::ReplaceRetWithJump {
                    range: token_lsp_range(ret_token),
                    new_text: format!("jr {condition_text},{qualified_label}")
                });

                tail_blocks.push_str(&format!(
                    "{bare_label}\n    waitnops {nop_count}\n    ret\n"
                ));
            }
        }
    }

    if !tail_blocks.is_empty() {
        // `range.end` already sits at the start of a real line (character
        // 0) whenever the selection ends exactly on a line boundary (the
        // common case, including every whole-line selection via
        // `stabilize_lines`) - use it directly. A genuine mid-line end (a
        // precise, partial selection) needs to skip to the *next* line
        // instead, so the new block always starts fresh, never mid-line.
        let at = if range.end.character == 0 {
            range.end
        }
        else {
            Position {
                line: range.end.line + 1,
                character: 0
            }
        };
        edits.push(StabilizeTextEdit::AppendTailBlocks {
            at,
            text: tail_blocks
        });
    }

    Ok(edits)
}

/// Whole-line convenience wrapper over [`stabilize_selection`] - covers
/// the entirety of both `start_line` and `end_line` (inclusive, 0-based),
/// ignoring any character position within them.
pub(super) fn stabilize_lines(
    listing: &LocatedListing,
    start_line: usize,
    end_line: usize
) -> Result<Vec<StabilizeTextEdit>, String> {
    stabilize_selection(
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

#[cfg(test)]
mod tests {
    use cpclib_asm::parser::parse_z80_str;

    use super::*;

    fn stabilize(text: &str) -> Result<Vec<StabilizeTextEdit>, String> {
        let listing = parse_z80_str(text).unwrap();
        let end_line = text.lines().count().saturating_sub(1);
        stabilize_lines(&listing, 0, end_line)
    }

    /// Only `InsertPadding` edits expected - collapses them to `(line,
    /// count)` pairs, sorted, for easy comparison. Panics if a rewrite/
    /// append edit shows up where a test isn't expecting one.
    fn insertions_only(edits: Vec<StabilizeTextEdit>) -> Vec<(usize, u32)> {
        let mut v: Vec<(usize, u32)> = edits
            .into_iter()
            .map(|e| {
                match e {
                    StabilizeTextEdit::InsertPadding { line, nop_count } => (line, nop_count),
                    _ => panic!("expected only InsertPadding edits")
                }
            })
            .collect();
        v.sort_unstable();
        v
    }

    /// The classic shape: `jr nz,.b` / cheap not-taken arm (ends with its
    /// own unconditional jump over) / `.b:` expensive taken arm / `.over:`.
    /// Hand-verified: taken path = 3 (jr taken) + 1 (ld a,b) + 1 (ld c,d) =
    /// 5; not-taken path = 2 (jr not taken) + 1 (ld a,b) + 3 (jr .over) = 6.
    /// The taken arm is cheaper by 1, so 1 NOP must land right before
    /// `.over:` (the end of the taken arm's own content).
    #[test]
    fn single_branch_pads_the_cheaper_arm() {
        let text = "    jr nz,.b\n    ld a,b\n    jr .over\n.b:\n    ld a,b\n    ld c,d\n.over:\n";
        let edits = insertions_only(stabilize(text).unwrap());
        assert_eq!(edits, vec![(6, 1)], "{edits:?}");
    }

    #[test]
    fn already_balanced_branch_needs_no_edits() {
        // not-taken: 2 (jr) + nop*2 (2) + jr .over (3) = 7
        // taken:      3 (jr) + nop*4 (4) = 7
        let text = "    jr nz,.b\n    nop\n    nop\n    jr .over\n.b:\n    nop\n    nop\n    nop\n    nop\n.over:\n";
        let edits = stabilize(text).unwrap();
        assert!(edits.is_empty(), "{}", edits.len());
    }

    /// A branch nested inside the *taken* arm of an outer branch - the
    /// inner branch must be resolved first (padding its own cheaper arm),
    /// and the outer branch's own balancing must treat the now-fixed inner
    /// region as a single cost, not double-count or ignore it.
    #[test]
    fn nested_branch_is_resolved_innermost_first() {
        let text = "\
    jr nz,.outer_b
    nop
    jr .outer_over
.outer_b:
    jr z,.inner_b
    nop
    jr .inner_over
.inner_b:
    nop
    nop
.inner_over:
.outer_over:
";
        let edits = insertions_only(stabilize(text).unwrap());
        // Inner branch: taken (3) + nop*2 (2) = 5; not-taken (2) + nop (1) +
        // jr .inner_over (3) = 6 -> inner taken arm padded by 1, landing
        // right before `.inner_over:` (line 10).
        assert!(edits.contains(&(10, 1)), "{edits:?}");
        // Whatever the outer branch's own imbalance resolves to, it must
        // land at or before `.outer_over:` (line 11), never past it.
        assert!(edits.iter().all(|&(line, _)| line <= 11), "{edits:?}");
    }

    /// Two independent, sequential branches in one selection - each must be
    /// balanced on its own, edits from both present.
    #[test]
    fn sibling_branches_are_each_balanced_independently() {
        let text = "\
    jr nz,.a_b
    nop
    jr .a_over
.a_b:
    nop
    nop
.a_over:
    jr z,.c_b
    nop
    jr .c_over
.c_b:
    nop
    nop
    nop
.c_over:
";
        let edits = insertions_only(stabilize(text).unwrap());
        // First branch: taken (3+2=5) vs not-taken (2+1+3=6) -> pad taken
        // arm by 1 before `.a_over:` (line 6).
        assert!(edits.contains(&(6, 1)), "{edits:?}");
        // Second branch: taken (3+3=6) vs not-taken (2+1+3=6) -> already
        // balanced, no edit for it.
        assert_eq!(edits.len(), 1, "{edits:?}");
    }

    #[test]
    fn a_backward_jump_is_rejected_as_a_loop() {
        let text = ".loop:\n    nop\n    jr nz,.loop\n";
        assert!(stabilize(text).is_err());
    }

    /// `DJNZ` is still out of scope - its taken arm is a loop, and padding an
    /// unknown iteration count means nothing.
    ///
    /// A **conditional** `CALL` is out of scope for a different, narrower
    /// reason: `data/timings.txt` gives `call ccc,nn` as "5 or 3", so its cost
    /// is a range, and balancing needs one exact number to pad against.
    #[test]
    fn djnz_is_rejected() {
        let text = "    djnz .x\n.x:\n    nop\n    ret\n";
        assert!(stabilize(text).is_err());
    }

    /// A **conditional** call is out of scope for a narrower reason than the
    /// old blanket rejection: `data/timings.txt` gives `call ccc,nn` as
    /// "5 or 3", so its cost is a range, and balancing needs one exact number
    /// to pad against.
    ///
    /// It has to sit inside a real branch to be reached at all - a selection
    /// whose only conditional thing is a `call cc` has no branch to balance,
    /// so the honest answer there is "no edits", not an error.
    #[test]
    fn a_conditional_call_inside_a_branch_arm_is_rejected() {
        let text = "\
    jr nz,.taken
    call nz,.small
    jr .over
.taken:
    nop
.over:
    ret
.small:
    nop
    ret
";
        assert!(stabilize(text).is_err());
    }

    /// An *unconditional* call to a routine with a single exact cost used to
    /// be rejected out of hand, so a selection containing any call could never
    /// be stabilized. It can be now: the routine's own cost is priced in, and
    /// the two arms are balanced against it.
    #[test]
    fn an_unconditional_call_to_an_exact_routine_no_longer_blocks_stabilizing() {
        let text = "\
    jr nz,.taken
    call .small
    jr .over
.taken:
    call .big
.over:
    ret
.small:
    nop
    ret
.big:
    nop
    nop
    ret
";
        let edits = stabilize(text)
            .expect("a call with a knowable cost must no longer abort the whole pass");
        assert!(
            !edits.is_empty(),
            "the arms differ by their routines' costs, so padding is needed"
        );
    }

    #[test]
    fn a_jump_target_outside_the_selection_is_rejected() {
        let text = "    jr nz,.elsewhere\n    nop\n";
        assert!(stabilize(text).is_err());
    }

    /// A branch sharing its line with another `:`-chained instruction - the
    /// old text-based version had to specially reject this shape; AST-based
    /// tokens make it structurally impossible to miss, since each
    /// `:`-segment already parses into its own separate token, so this now
    /// balances normally instead of erroring.
    #[test]
    fn a_jump_sharing_its_line_with_other_instructions_still_balances() {
        let text = "    ld a,b : jr nz,.b\n    jr .over\n.b:\n    nop\n.over:\n";
        let edits = insertions_only(stabilize(text).unwrap());
        // not-taken: 2 (jr) + 3 (jr .over) = 5; taken: 3 (jr) + 1 (nop) = 4
        // -> pad taken arm by 1 before `.over:` (line 4: `ld a,b : jr
        // nz,.b`(0), `jr .over`(1), `.b:`(2), `nop`(3), `.over:`(4)).
        assert_eq!(edits, vec![(4, 1)], "{edits:?}");
    }

    #[test]
    fn a_selection_with_no_branch_at_all_yields_no_edits() {
        let text = "    ld a,b\n    ld c,d\n    nop\n";
        let edits = stabilize(text).unwrap();
        assert!(edits.is_empty(), "{}", edits.len());
    }

    /// The user's own real-world idiom: a routine that returns early via
    /// `ret nc`, then continues and ends in a plain `ret`. This used to
    /// blanket-reject (RET was entirely unsupported) - the actual root
    /// cause of the Quick Fix never appearing for real selections. Now it
    /// rewrites the early-exit `ret nc` into a qualified jump and appends a
    /// padded tail block.
    #[test]
    fn conditional_ret_early_exit_arm_is_rewritten_and_padded() {
        let text = "\
bc26_hl
    ld a,h
    add 8
    ld h,a
    ret nc
    ld bc,0xc000 + 96
    add hl,bc
    ret
";
        let edits = stabilize(text).unwrap();
        assert_eq!(edits.len(), 2, "{}", edits.len());

        let replace = edits
            .iter()
            .find_map(|e| {
                match e {
                    StabilizeTextEdit::ReplaceRetWithJump { range, new_text } => {
                        Some((range, new_text))
                    },
                    _ => None
                }
            })
            .expect("expected a ReplaceRetWithJump edit");
        assert_eq!(replace.1, "jr nc,bc26_hl.__BASM__stabilize_pad_1");
        // `ret nc` sits at line 4, columns 4..10 (0-based: "    ret nc").
        assert_eq!(replace.0.start, Position::new(4, 4));
        assert_eq!(replace.0.end, Position::new(4, 10));

        let append = edits
            .iter()
            .find_map(|e| {
                match e {
                    StabilizeTextEdit::AppendTailBlocks { at, text } => Some((at, text)),
                    _ => None
                }
            })
            .expect("expected an AppendTailBlocks edit");
        assert_eq!(*append.0, Position::new(8, 0));
        // Hand-verified against real timing data (`ret nc`: taken=4,
        // not-taken=2; `jr nc,x`: taken=3, not-taken=2; unconditional
        // `ret`=3): not-taken path = 2 + 3 (ld bc,nn) + 3 (add hl,bc) + 3
        // (ret) = 11; taken path once rewritten = 3 (jr taken) + N +
        // 3 (the newly appended ret) - solving 3+N+3=11 gives N=5.
        assert_eq!(
            append.1,
            ".__BASM__stabilize_pad_1\n    waitnops 5\n    ret\n"
        );
    }

    /// If the selection doesn't include the routine's own global label,
    /// there's nothing to qualify the synthesized local label against -
    /// the rewrite must be refused rather than emit an unqualified
    /// reference that might not reliably resolve.
    #[test]
    fn conditional_ret_rewrite_without_an_enclosing_global_label_is_refused() {
        let text = "    ret nc\n    ld bc,10\n    add hl,bc\n    ret\n";
        assert!(stabilize(text).is_err());
    }

    /// `stabilize_selection` at precise-character granularity: a mid-line
    /// range should behave identically to the whole-line wrapper when it
    /// happens to span exactly the same content, but must *exclude* a
    /// token sitting outside the given character bounds even on an
    /// otherwise-included line.
    #[test]
    fn stabilize_selection_respects_precise_character_bounds() {
        let text =
            "junk: jr nz,.b\n    ld a,b\n    jr .over\n.b:\n    ld a,b\n    ld c,d\n.over:\n";
        let listing = parse_z80_str(text).unwrap();
        // Start the selection right at "jr nz,.b" (column 6), excluding
        // the leading "junk:" label - which must not confuse the balancer
        // or the owning-global-label scan.
        let edits = stabilize_selection(
            &listing,
            Range {
                start: Position::new(0, 6),
                end: Position::new(7, 0)
            }
        )
        .unwrap();
        let insertions = insertions_only(edits);
        assert_eq!(insertions, vec![(6, 1)], "{insertions:?}");
    }
}
