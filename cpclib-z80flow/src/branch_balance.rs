//! Automatic branch-timing stabilization: detects hand-written runtime
//! conditional branches (`JR`/`JP` with a flag condition, and `RET` with a
//! flag condition) in a token sequence, builds their control-flow graph
//! (via `crate::cfg`), and computes the padding needed so every path from a
//! branch to its merge point costs the same - working bottom-up, innermost
//! branch first, so nested/sequential branches all end up correctly
//! balanced.
//!
//! Generic over `T: ListingElement` (so it works identically on plain
//! `Token` and the parser's own `LocatedToken`) and over the cost source
//! (a caller-supplied `Fn(&T) -> InstructionCost`) - this module carries no
//! Z80 timing-data table of its own. The one that already exists,
//! `cpclib-lsp`'s `timing.rs`, stays the single source of truth for the
//! LSP's own use; any other consumer (a future `basm` CLI, say) can supply
//! its own cost source without this module changing at all. Operates on
//! plain token *indices* within the slice it's given, never on source
//! spans/positions - a caller with real `LocatedToken`s can always recover
//! a genuine source line from a returned index via that token's own
//! `MayHaveSpan`, once the tokens it passed in are known to still be in
//! their original document order.
//!
//! `RET cc` is modeled as a branch whose *taken* side goes straight to the
//! selection's own virtual exit - see `crate::cfg`'s own module doc comment
//! for why. When the *cheaper* arm turns out to be that direct-to-exit
//! taken side, there is no in-selection code left to pad into -
//! `balance_branches` reports `StabilizeEdit::RewriteConditionalRetAndPad`
//! instead of `InsertPadding` for that case; the caller is expected to
//! rewrite the `RET cc` into `JR cc,<fresh label>` and append a new padded
//! tail block (this module has no opinion on label text or the exact
//! padding instruction used - purely structural, see `StabilizeEdit`'s own
//! doc comment).
//!
//! **Out of scope for v1** (each rejected with a specific reason, never
//! silently mishandled) - `crate::cfg::build_cfg` itself is permissive
//! (shared with `cost_range`, which needs to degrade gracefully instead of
//! rejecting); the rejections below are this module's own validation,
//! applied right after building the CFG:
//! - Loops (any jump target at or before its own index - a back-edge). A
//!   loop's whole point is a variable iteration count; there is no single
//!   "path cost" to equalize in the same sense a straight branch-and-merge
//!   has.
//! - `DJNZ`/`CALL` (conditional or not, unlike `RET`). Their "taken" arm
//!   isn't a self-contained, in-selection instruction sequence the way
//!   `RET`'s "leaves to the exit" cheat can express: `DJNZ`'s taken arm is
//!   a loop, and a taken `CALL` runs an entire subroutine of unknowable
//!   cost before returning.
//! - Any jump whose target label isn't defined inside `tokens` itself
//!   ("escapes") - balancing needs both arms fully in view.
//! - `IF`/`ELSE`/`ENDIF` isn't a runtime branch at all (a compile-time
//!   construct - the assembler emits exactly one arm's bytes), so it's
//!   simply invisible here, not specially rejected.

use std::collections::HashMap;

use cpclib_tokens::{ListingElement, Mnemonic};

use crate::cfg::{Cfg, Terminator, build_cfg, compute_postdominators, expect_block, mnemonic_of};

/// A cost source the algorithm queries once per token - kept fully
/// decoupled from any specific timing-data representation (see the module
/// doc comment).
pub enum InstructionCost {
    /// A plain instruction's single cost.
    Fixed(u32),
    /// A conditional `JR`/`JP`'s own two costs. Mirrors `timing::
    /// format_hover`'s "taken/not taken" convention in `cpclib-lsp`.
    Conditional { taken: u32, not_taken: u32 },
    /// The cost source doesn't recognize this instruction - aborts the
    /// whole pass (same fail-safe policy as every other unsupported case).
    Unknown
}

/// One point where padding is needed, in one of two shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilizeEdit {
    /// Insert `nop_count` cost-units' worth of padding *before*
    /// `insert_before_index` (an index into the same `tokens` slice
    /// `balance_branches` was given) - never after, see
    /// `arm_padding_index`'s own doc comment for why that matters. Used
    /// whenever the cheaper arm has real in-selection code to pad into.
    InsertPadding {
        insert_before_index: usize,
        nop_count: u32
    },
    /// `tokens[ret_token_index]` is a conditional `RET` whose *taken*
    /// (early-exit) arm is the cheaper one, and - unlike every other
    /// branch shape this module balances - that arm has no in-selection
    /// code of its own to pad into, since it leaves the routine
    /// immediately. The caller must rewrite that `RET cc` into `JR
    /// cc,<fresh label>` and append a new tail block (`<fresh label>` /
    /// `nop_count` worth of padding / `RET`) - this module has no opinion
    /// on the exact label text or padding instruction(s) used, purely
    /// structural.
    RewriteConditionalRetAndPad {
        ret_token_index: usize,
        nop_count: u32
    }
}

impl StabilizeEdit {
    /// The token index this edit is anchored to, used only to keep
    /// `balance_branches`'s returned edits in a consistent (descending)
    /// order.
    fn anchor_index(&self) -> usize {
        match *self {
            StabilizeEdit::InsertPadding {
                insert_before_index,
                ..
            } => insert_before_index,
            StabilizeEdit::RewriteConditionalRetAndPad {
                ret_token_index, ..
            } => ret_token_index
        }
    }
}

/// `Err` the first time `tokens` contains a `CALL` or `DJNZ` anywhere, in
/// any form - its taken arm is a loop, with no self-contained, in-selection
/// cost this module could express, and padding an unknown iteration count is
/// not meaningful.
///
/// **`CALL` used to be rejected here too**, on the grounds that "a taken
/// `CALL` runs an entire subroutine of unknowable cost". That is no longer
/// true: `cost_range::exact_call_cost` prices the callee when the callee is in
/// view and costs a single known number, which is the common case for a
/// routine defined in the same file. A call whose cost is *not* exact - it
/// branches, loops, recurses, or lives elsewhere - still stops the balance,
/// but now at `straight_line_cost`, with a reason naming the actual problem
/// rather than the mnemonic.
///
/// Checked directly against `tokens`, not the `Cfg` - `crate::cfg::build_cfg`
/// itself doesn't treat `DJNZ` specially (it's permissive, shared with
/// `cost_range`, which *does* want to model it), so this module's own
/// rejection has to be an explicit, separate scan.
fn reject_djnz<T: ListingElement>(tokens: &[&T]) -> Result<(), String> {
    for (i, token) in tokens.iter().enumerate() {
        if let Some(m @ Mnemonic::Djnz) = mnemonic_of(*token) {
            return Err(format!(
                "{m:?} at token index {i} isn't supported (only JR/JP/RET are balanced)"
            ));
        }
    }
    Ok(())
}

/// Sums costs for `tokens[start..=end]` using each instruction's single
/// (`Fixed`) cost, or the *taken* cost for anything still `Conditional` at
/// this point - by the time this is called every branch inside the range
/// has already been resolved to a fixed cost by an earlier, innermost
/// step, so nothing left in range should still be genuinely dual-valued;
/// this is a defensive fallback, not expected to matter in practice.
/// Label tokens contribute nothing (a real, zero-cost marker).
fn straight_line_cost<T: ListingElement>(
    tokens: &[&T],
    labels: &HashMap<String, usize>,
    start: usize,
    end: usize,
    cost: &impl Fn(&T) -> InstructionCost
) -> Result<u32, String> {
    if start > end {
        return Ok(0);
    }
    let mut total = 0u32;
    for token in tokens[start..=end].iter().copied() {
        if token.is_label() {
            continue;
        }
        // A `call`'s cost is its own plus its callee's - and balancing needs
        // that to be one exact number, since it pads against the difference.
        if mnemonic_of(token) == Some(Mnemonic::Call) {
            let Some(n) = crate::cost_range::exact_call_cost(tokens, labels, token, cost)
            else {
                return Err(
                    "a CALL in the selection has no single known cost (its routine \
                     branches, loops, or is defined outside the selection)"
                        .to_string()
                );
            };
            total += n;
            continue;
        }
        match cost(token) {
            InstructionCost::Fixed(n) => total += n,
            InstructionCost::Conditional { taken, .. } => total += taken,
            InstructionCost::Unknown => {
                return Err("an instruction in the selection has no known cost".to_string());
            }
        }
    }
    Ok(total)
}

/// The insertion index (0-based, into the same `tokens` slice) and NOP
/// count needed to balance every branch in `cfg`, processed from the
/// last-appearing branch to the first - since a branch nested inside an
/// earlier branch's arm necessarily starts on a later index, this order
/// always resolves an inner branch before the outer branch that contains
/// it, exactly the bottom-up order the algorithm needs.
fn balance<T: ListingElement>(
    cfg: &Cfg,
    postdom: &[usize],
    tokens: &[&T],
    labels: &HashMap<String, usize>,
    cost: &impl Fn(&T) -> InstructionCost
) -> Result<Vec<StabilizeEdit>, String> {
    let mut edits = Vec::new();
    // resolved[b] = fixed cost from block b to postdom[b], once b (a
    // branch block) has been balanced. Plain blocks are never keyed here.
    let mut resolved: HashMap<usize, u32> = HashMap::new();

    let branch_indices: Vec<usize> = (0..cfg.blocks.len())
        .rev()
        .filter(|&i| matches!(cfg.terms[i], Terminator::Branch { .. }))
        .collect();

    for &b in &branch_indices {
        let Terminator::Branch {
            taken, not_taken, ..
        } = &cfg.terms[b]
        else {
            unreachable!()
        };
        // Already validated forward-only by the caller - safe to unwrap.
        let taken = expect_block(taken);
        let not_taken = *not_taken;

        // Real taken/not-taken cost of the branch instruction itself,
        // supplied by the caller's cost function (see `build_cfg`'s own
        // note on why this is deferred to here).
        let branch_token = tokens[cfg.blocks[b].end];
        let (cost_taken, cost_not_taken) = match cost(branch_token) {
            InstructionCost::Conditional { taken, not_taken } => (taken, not_taken),
            InstructionCost::Fixed(n) => (n, n),
            InstructionCost::Unknown => {
                return Err("the branch instruction itself has no known cost".to_string());
            }
        };
        let merge = postdom[b];

        let taken_cost =
            cost_taken + arm_cost(cfg, &resolved, postdom, tokens, labels, cost, taken, merge)?;
        let not_taken_cost =
            cost_not_taken + arm_cost(cfg, &resolved, postdom, tokens, labels, cost, not_taken, merge)?;

        if taken_cost != not_taken_cost {
            let padding_taken_arm = taken_cost < not_taken_cost;
            let (cheaper_arm_start, delta) = if padding_taken_arm {
                (taken, not_taken_cost - taken_cost)
            }
            else {
                (not_taken, taken_cost - not_taken_cost)
            };

            // `taken` only ever equals `cfg.exit()` directly for a
            // conditional RET's own early-exit side (a JR/JP branch's
            // taken side always resolves to a real block via
            // `resolve_successor`) - that's the one shape with no
            // in-selection code to insert padding into. `not_taken` can
            // *also* equal `cfg.exit()` (e.g. a RET cc as the very last
            // token, nothing follows it in the selection), but that case
            // still has a well-defined, correct insertion point via
            // `arm_padding_index`'s own "end == exit" fallback - only the
            // taken side needs the rewrite treatment.
            if padding_taken_arm && cheaper_arm_start == cfg.exit() {
                edits.push(StabilizeEdit::RewriteConditionalRetAndPad {
                    ret_token_index: cfg.blocks[b].end,
                    nop_count: delta
                });
            }
            else {
                edits.push(StabilizeEdit::InsertPadding {
                    insert_before_index: arm_padding_index(
                        cfg,
                        &resolved,
                        postdom,
                        cheaper_arm_start,
                        merge
                    ),
                    nop_count: delta
                });
            }
        }

        resolved.insert(b, taken_cost.max(not_taken_cost));
    }

    Ok(edits)
}

/// Cost from `start` to `end` (exclusive), walking the single effective
/// successor chain: a plain block contributes its own straight-line cost
/// and moves to its one successor; an already-`resolved` branch block
/// contributes its fixed combined cost and skips straight to its own
/// post-dominator (everything in between was already counted inside that
/// resolved cost).
#[allow(clippy::too_many_arguments)]
fn arm_cost<T: ListingElement>(
    cfg: &Cfg,
    resolved: &HashMap<usize, u32>,
    postdom: &[usize],
    tokens: &[&T],
    labels: &HashMap<String, usize>,
    cost: &impl Fn(&T) -> InstructionCost,
    start: usize,
    end: usize
) -> Result<u32, String> {
    let mut total = 0u32;
    let mut cur = start;
    while cur != end {
        if cur == cfg.exit() {
            return Err("branch arm never reaches its expected merge point".to_string());
        }
        if let Some(&c) = resolved.get(&cur) {
            total += c;
            cur = postdom[cur];
        }
        else {
            let block = cfg.blocks[cur];
            total += straight_line_cost(tokens, labels, block.start, block.end, cost)?;
            cur = match &cfg.terms[cur] {
                Terminator::Fallthrough(t) => *t,
                Terminator::Jump(s) => expect_block(s),
                Terminator::Branch { .. } => {
                    unreachable!(
                        "branch blocks are always inserted into `resolved` before being walked past"
                    )
                }
            };
        }
    }
    Ok(total)
}

/// The 0-based index *before which* to insert this arm's padding NOPs, so
/// they execute unconditionally along the arm's real runtime path. Walks
/// the arm the same way `arm_cost` does (skipping straight over any
/// already-resolved nested branch to its own post-dominator) and stops at
/// whichever comes first:
/// - a block whose own last token *is* a jump (`Terminator::Jump`) - the
///   padding must land **before** that jump, never after it, or it would
///   be unreachable dead code (the jump has already unconditionally left
///   by then);
/// - the arm reaching `merge` via plain fallthrough with no jump of its
///   own - the padding lands right before `merge`'s own first token, i.e.
///   at the very end of this arm's content.
fn arm_padding_index(
    cfg: &Cfg,
    resolved: &HashMap<usize, u32>,
    postdom: &[usize],
    start: usize,
    end: usize
) -> usize {
    let mut cur = start;
    loop {
        if cur == end || cur == cfg.exit() {
            return if end == cfg.exit() {
                cfg.blocks.last().map(|b| b.end + 1).unwrap_or(0)
            }
            else {
                cfg.blocks[end].start
            };
        }
        if resolved.contains_key(&cur) {
            cur = postdom[cur];
            continue;
        }
        match cfg.terms[cur] {
            Terminator::Jump(_) => return cfg.blocks[cur].end,
            Terminator::Fallthrough(t) => cur = t,
            Terminator::Branch { .. } => {
                unreachable!(
                    "branch blocks are always inserted into `resolved` before being walked past"
                )
            }
        }
    }
}

/// Detects and balances every hand-written `JR`/`JP`/`RET` branch in
/// `tokens`, returning the padding needed (empty when already balanced).
/// `Err` with a human-readable reason when `tokens` contains something
/// this v1 doesn't support (a loop, `DJNZ`, a `CALL` whose routine has no
/// single exact cost, or an escaping jump target) - the caller is expected to offer no action at
/// all in that case, not surface the message as an error (this isn't
/// necessarily a mistake, just a shape this feature doesn't cover yet).
pub fn balance_branches<T: ListingElement>(
    tokens: &[&T],
    cost: impl Fn(&T) -> InstructionCost
) -> Result<Vec<StabilizeEdit>, String> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    reject_djnz(tokens)?;
    let cfg = build_cfg(tokens)?;
    let labels = crate::cfg::label_indices(tokens);
    cfg.validate_forward_only()?;
    if !cfg
        .terms
        .iter()
        .any(|t| matches!(t, Terminator::Branch { .. }))
    {
        return Ok(Vec::new());
    }
    let postdom = compute_postdominators(&cfg);
    let mut edits = balance(&cfg, &postdom, tokens, &labels, &cost)?;
    edits.sort_unstable_by(|a, b| b.anchor_index().cmp(&a.anchor_index()));
    Ok(edits)
}
