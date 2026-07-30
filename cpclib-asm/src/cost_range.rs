//! Min/max cost of the distinct runtime paths through a token selection -
//! the read-only counterpart to `branch_balance`'s padding calculation,
//! sharing the same permissive `crate::cfg` construction (see that
//! module's own doc comment for why one shared, permissive CFG beats two
//! parallel builders).
//!
//! Unlike `branch_balance::balance_branches`, this never rejects a loop,
//! an escaping jump target, `DJNZ`, or `CALL` - it degrades gracefully
//! instead, since it's an informational query, not a code-modifying
//! action:
//! - A loop (`DJNZ`, or a backward `JR`/`JP`) sets `unbounded = true` (the
//!   real worst case is unknowable - an unknown iteration count), but
//!   still reports a real, meaningful `min` (the loop body's own
//!   single-pass cost - the loop-exit/not-taken side is the only path
//!   that actually continues toward this selection's own exit; the
//!   looping/taken side goes backward, not forward, so it never
//!   contributes a *competing* min/max candidate the way a real forward
//!   branch does).
//! - `CALL` isn't a terminator at all (see `crate::cfg`'s own doc comment)
//!   - it just contributes its own known, fixed instruction cost like any
//!     other straight-line instruction.
//! - An escaping jump target (not defined anywhere in the given tokens)
//!   simply means that one path's cost stops being trackable at the point
//!   of escape - a well-defined partial cost, not an error.
//! - `InstructionCost::Unknown` increments `unrecognized_count` and
//!   contributes 0, rather than aborting the whole computation.

use cpclib_tokens::ListingElement;

pub use crate::branch_balance::InstructionCost;
use crate::cfg::{Successor, Terminator, build_cfg};

/// The min/max cost summary for a token selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CostRange {
    /// The cheapest real runtime path's total cost.
    pub min: u32,
    /// The costliest real runtime path's total cost - meaningless as a
    /// real upper bound when `unbounded` is true (a partial sum, not the
    /// actual worst case), mirroring `cpclib-lsp`'s own pre-existing
    /// `SelectionCycleCount::max_nops` convention for the same reason.
    pub max: u32,
    /// True when some loop's real iteration count isn't statically known
    /// here, making `max` a partial sum rather than a real upper bound.
    pub unbounded: bool,
    pub instruction_count: u32,
    /// Recognized-as-an-instruction tokens whose cost source didn't know a
    /// timing for them - `min`/`max` are lower bounds when this is nonzero.
    pub unrecognized_count: u32
}

/// Sums `cost()` over `tokens[range]`, skipping labels, tracking
/// `instruction_count`/`unrecognized_count` as it goes. Uses a
/// `Conditional` cost's *taken* value as a defensive fallback (matches
/// `branch_balance::straight_line_cost`'s own convention) - by design,
/// this is only ever called on a range that doesn't include a block's own
/// diverging branch instruction (that one is priced separately, see
/// `cost_range`'s own `Terminator::Branch` handling).
fn sum_range<T: ListingElement>(
    tokens: &[&T],
    range: std::ops::Range<usize>,
    cost: &impl Fn(&T) -> InstructionCost,
    instruction_count: &mut u32,
    unrecognized_count: &mut u32
) -> u32 {
    let mut total = 0u32;
    for token in tokens[range].iter().copied() {
        if token.is_label() {
            continue;
        }
        match cost(token) {
            // `Fixed(0)` is this whole feature's own established signal
            // for "not really an executing instruction at all" (a
            // directive, e.g.) - no real Z80 instruction genuinely costs
            // zero nops, so this is a safe, domain-specific heuristic, not
            // a fragile coincidence. Not counted as a real instruction,
            // mirroring how a label is skipped outright just above.
            InstructionCost::Fixed(0) => {},
            InstructionCost::Fixed(n) => {
                total += n;
                *instruction_count += 1;
            },
            InstructionCost::Conditional { taken, .. } => {
                total += taken;
                *instruction_count += 1;
            },
            InstructionCost::Unknown => {
                *unrecognized_count += 1;
            }
        }
    }
    total
}

/// The min/max cost of every distinct runtime path through `tokens`, from
/// its first token to wherever each path terminates (the selection's own
/// virtual exit, or an escape point). `Err` only for a genuine
/// parse-shape anomaly `crate::cfg::build_cfg` itself can't interpret at
/// all (e.g. a `JR`/`JP`/`DJNZ` whose operand isn't recognizable as a
/// label) - not a policy choice, every other ambiguity is handled
/// gracefully (see the module doc comment).
pub fn cost_range<T: ListingElement>(
    tokens: &[&T],
    cost: impl Fn(&T) -> InstructionCost
) -> Result<CostRange, String> {
    if tokens.is_empty() {
        return Ok(CostRange::default());
    }
    let cfg = build_cfg(tokens)?;
    let exit = cfg.exit();

    let mut best_min = vec![0u32; exit + 1];
    let mut best_max = vec![0u32; exit + 1];
    let mut block_unbounded = vec![false; exit + 1];
    let mut instruction_count = 0u32;
    let mut unrecognized_count = 0u32;

    for i in (0..cfg.blocks.len()).rev() {
        let block = cfg.blocks[i];
        match &cfg.terms[i] {
            Terminator::Fallthrough(next) => {
                let own = sum_range(
                    tokens,
                    block.start..block.end + 1,
                    &cost,
                    &mut instruction_count,
                    &mut unrecognized_count
                );
                best_min[i] = own + best_min[*next];
                best_max[i] = own + best_max[*next];
                block_unbounded[i] = block_unbounded[*next];
            },
            Terminator::Jump(successor) => {
                let own = sum_range(
                    tokens,
                    block.start..block.end + 1,
                    &cost,
                    &mut instruction_count,
                    &mut unrecognized_count
                );
                match successor {
                    Successor::Block(next) => {
                        best_min[i] = own + best_min[*next];
                        best_max[i] = own + best_max[*next];
                        block_unbounded[i] = block_unbounded[*next];
                    },
                    Successor::Loop { .. } => {
                        // An unconditional backward jump never reaches
                        // this selection's own exit via this path at all
                        // (as far as a static pass can tell) - only this
                        // block's own visible cost is well-defined.
                        best_min[i] = own;
                        best_max[i] = own;
                        block_unbounded[i] = true;
                    },
                    Successor::Escapes { .. } => {
                        best_min[i] = own;
                        best_max[i] = own;
                        block_unbounded[i] = false;
                    }
                }
            },
            Terminator::Branch {
                taken, not_taken, ..
            } => {
                // The prefix (everything before the branch instruction
                // itself, within the same block) is shared by both sides
                // unconditionally - only the branch instruction's own
                // cost diverges.
                let prefix = sum_range(
                    tokens,
                    block.start..block.end,
                    &cost,
                    &mut instruction_count,
                    &mut unrecognized_count
                );
                let branch_token = tokens[block.end];
                let (branch_taken_cost, branch_not_taken_cost) = match cost(branch_token) {
                    InstructionCost::Conditional { taken, not_taken } => {
                        instruction_count += 1;
                        (taken, not_taken)
                    },
                    InstructionCost::Fixed(n) => {
                        instruction_count += 1;
                        (n, n)
                    },
                    InstructionCost::Unknown => {
                        unrecognized_count += 1;
                        (0, 0)
                    }
                };

                let not_taken_min = branch_not_taken_cost + best_min[*not_taken];
                let not_taken_max = branch_not_taken_cost + best_max[*not_taken];
                let not_taken_unbounded = block_unbounded[*not_taken];

                let (min, max, unbounded) = match taken {
                    Successor::Block(t) => {
                        let taken_min = branch_taken_cost + best_min[*t];
                        let taken_max = branch_taken_cost + best_max[*t];
                        (
                            taken_min.min(not_taken_min),
                            taken_max.max(not_taken_max),
                            block_unbounded[*t] || not_taken_unbounded
                        )
                    },
                    Successor::Loop { .. } => {
                        // The taken side loops backward, never forward
                        // toward this selection's own exit - only
                        // not_taken (the loop-exit path) is a real
                        // candidate; see the module doc comment for why
                        // the loop itself still contributes a real `min`
                        // (one pass through the loop body, already
                        // reflected in reaching this branch at all).
                        (not_taken_min, not_taken_max, true)
                    },
                    Successor::Escapes { .. } => {
                        // The taken side leaves the selection with just
                        // its own known cost - a well-defined competing
                        // candidate, not an error.
                        (
                            branch_taken_cost.min(not_taken_min),
                            branch_taken_cost.max(not_taken_max),
                            not_taken_unbounded
                        )
                    }
                };
                best_min[i] = prefix + min;
                best_max[i] = prefix + max;
                block_unbounded[i] = unbounded;
            }
        }
    }

    Ok(CostRange {
        min: best_min[0],
        max: best_max[0],
        unbounded: block_unbounded[0],
        instruction_count,
        unrecognized_count
    })
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::{DataAccessElem, ListingElement, Mnemonic};

    use super::*;
    use crate::parser::obtained::LocatedToken;
    use crate::parser::parse_z80_str;

    /// A tiny, test-only cost source - deliberately distinct values from
    /// `branch_balance`'s own test cost function, to keep every hand
    /// derivation below self-contained and easy to re-check independently.
    fn test_cost(token: &LocatedToken) -> InstructionCost {
        match token.mnemonic() {
            Some(Mnemonic::Jr) | Some(Mnemonic::Ret) => {
                if token.mnemonic_arg1().is_some_and(|a| a.is_flag_test()) {
                    InstructionCost::Conditional {
                        taken: 3,
                        not_taken: 2
                    }
                }
                else {
                    InstructionCost::Fixed(3)
                }
            },
            Some(Mnemonic::Djnz) => {
                InstructionCost::Conditional {
                    taken: 3,
                    not_taken: 2
                }
            },
            Some(Mnemonic::Ld) | Some(Mnemonic::Nop) => InstructionCost::Fixed(1),
            Some(Mnemonic::Call) => InstructionCost::Fixed(5),
            None => InstructionCost::Fixed(0),
            _ => InstructionCost::Unknown
        }
    }

    fn range(code: &str) -> Result<CostRange, String> {
        let listing = parse_z80_str(code).unwrap();
        let tokens: Vec<&LocatedToken> = listing.iter().collect();
        cost_range(&tokens, test_cost)
    }

    /// A diamond with a real prefix *before* the branch, proving the
    /// prefix is actually included in both totals (not just the delta, the
    /// way `branch_balance::balance` gets to skip it). Hand-verified:
    /// taken = prefix(1) + jr_taken(3) + ld d,e(1) + ld h,l(1) + ret(3) =
    /// 9; not-taken = prefix(1) + jr_not_taken(2) + ld c,d(1) + jr
    /// .over(3) + ret(3) = 10.
    #[test]
    fn a_diamond_with_a_real_prefix_reports_absolute_totals() {
        let code = "\
    ld a,b
    jr nz,.b
    ld c,d
    jr .over
.b
    ld d,e
    ld h,l
.over
    ret
";
        let summary = range(code).unwrap();
        assert_eq!(
            summary,
            CostRange {
                min: 9,
                max: 10,
                unbounded: false,
                instruction_count: 7,
                unrecognized_count: 0
            }
        );
    }

    /// A `DJNZ` loop: unbounded (an unknown iteration count), but the
    /// minimum is still real and well-defined - one pass through the loop
    /// body. Hand-verified: min = ld a,b(1) + nop(1) + djnz not-taken(2) +
    /// ret(3) = 7 (the loop's *taken*/looping side never competes as a
    /// forward candidate at all - see the module doc comment).
    #[test]
    fn a_djnz_loop_is_unbounded_with_a_real_minimum() {
        let code = "\
    ld a,b
.loop
    nop
    djnz .loop
    ret
";
        let summary = range(code).unwrap();
        assert_eq!(summary.min, 7, "{summary:?}");
        assert!(summary.unbounded, "{summary:?}");
    }

    /// `CALL` folds into straight-line cost using only its own known
    /// instruction cost - it doesn't split the CFG, and doesn't stop a
    /// real branch elsewhere in the same selection from being modeled
    /// correctly. Hand-verified: taken = jr_taken(3) + nop(1) = 4;
    /// not-taken = jr_not_taken(2) + call(5) + jr .over(3) = 10.
    #[test]
    fn call_contributes_its_own_cost_without_splitting_the_cfg() {
        let code = "\
    jr nz,.b
    call foo
    jr .over
.b
    nop
.over
";
        let summary = range(code).unwrap();
        assert_eq!(summary.min, 4, "{summary:?}");
        assert_eq!(summary.max, 10, "{summary:?}");
        assert!(!summary.unbounded, "{summary:?}");
    }

    /// An escaping jump target (not defined anywhere in the selection) is
    /// a well-defined partial cost, not an error - that path simply stops
    /// being trackable at the point of escape. Hand-verified: taken
    /// (escapes) = jr_taken(3) alone; not-taken = jr_not_taken(2) +
    /// nop(1) = 3. Both sides happen to total 3 here.
    #[test]
    fn an_escaping_target_is_a_well_defined_partial_cost_not_an_error() {
        let code = "    jr nz,.elsewhere\n    nop\n";
        let summary = range(code).unwrap();
        assert_eq!(summary.min, 3, "{summary:?}");
        assert_eq!(summary.max, 3, "{summary:?}");
        assert!(!summary.unbounded, "{summary:?}");
    }

    /// An unrecognized instruction (`halt`, deliberately mapped to
    /// `Unknown` by this test's own cost function) increments
    /// `unrecognized_count` and contributes 0, rather than aborting.
    #[test]
    fn an_unrecognized_instruction_increments_the_count_instead_of_aborting() {
        let code = "    nop\n    halt\n    nop\n";
        let summary = range(code).unwrap();
        assert_eq!(summary.min, 2, "{summary:?}");
        assert_eq!(summary.max, 2, "{summary:?}");
        assert_eq!(summary.unrecognized_count, 1, "{summary:?}");
        assert_eq!(summary.instruction_count, 2, "{summary:?}");
    }

    #[test]
    fn a_selection_with_no_branch_at_all_is_a_single_fixed_total() {
        let code = "    ld a,b\n    ld c,d\n    nop\n";
        let summary = range(code).unwrap();
        assert_eq!(summary.min, 3, "{summary:?}");
        assert_eq!(summary.max, 3, "{summary:?}");
        assert!(!summary.unbounded, "{summary:?}");
    }

    #[test]
    fn an_empty_selection_is_a_zero_total() {
        let listing = parse_z80_str("").unwrap();
        let tokens: Vec<&LocatedToken> = listing.iter().collect();
        let summary = cost_range(&tokens, test_cost).unwrap();
        assert_eq!(summary, CostRange::default());
    }
}
