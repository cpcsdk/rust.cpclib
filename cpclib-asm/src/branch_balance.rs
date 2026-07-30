//! Automatic branch-timing stabilization: detects hand-written runtime
//! conditional branches (`JR`/`JP` with a flag condition, and `RET` with a
//! flag condition) in a token sequence, builds their control-flow graph,
//! and computes the padding needed so every path from a branch to its
//! merge point costs the same - working bottom-up, innermost branch first,
//! so nested/sequential branches all end up correctly balanced.
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
//! selection's own virtual exit (a real routine's early-return really does
//! leave to an unrelated call site, but for balancing purposes every path
//! through the routine converges there regardless of which `RET` fires, so
//! treating "taken" as reaching that shared exit directly - rather than a
//! real label - gives the correct answer without needing to know anything
//! about the caller). An unconditional trailing `RET` is likewise just a
//! plain jump to the virtual exit. When the *cheaper* arm turns out to be
//! that direct-to-exit taken side, there is no in-selection code left to
//! pad into - `balance_branches` reports `StabilizeEdit::
//! RewriteConditionalRetAndPad` instead of `InsertPadding` for that case;
//! the caller is expected to rewrite the `RET cc` into `JR cc,<fresh
//! label>` and append a new padded tail block (this module has no opinion
//! on label text or the exact padding instruction used - purely
//! structural, see `StabilizeEdit`'s own doc comment).
//!
//! **Out of scope for v1** (each rejected with a specific reason, never
//! silently mishandled):
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

use cpclib_tokens::{DataAccessElem, ExprElement, ListingElement, Mnemonic};

/// The only mnemonics this module treats as a two-target branch (`RET` is
/// handled separately, see `build_cfg`, since its conditional form has no
/// label operand at all).
const JUMP_MNEMONICS: &[Mnemonic] = &[Mnemonic::Jr, Mnemonic::Jp];
/// Mnemonics that make a selection unsupported wherever they appear, in
/// every form - their "taken" arm isn't expressible even via `RET`'s
/// direct-to-exit cheat.
const UNSUPPORTED_MNEMONICS: &[Mnemonic] = &[Mnemonic::Djnz, Mnemonic::Call];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BasicBlock {
    start: usize,
    end: usize // inclusive
}

#[derive(Debug, Clone, Copy)]
enum Terminator {
    Fallthrough(usize),
    Jump(usize),
    Branch {
        taken: usize,
        not_taken: usize,
        cost_taken: u32,
        cost_not_taken: u32
    }
}

struct Cfg {
    blocks: Vec<BasicBlock>,
    /// One terminator per real block; index `blocks.len()` is a virtual
    /// exit node every terminator target that falls off the end of
    /// `tokens` resolves to, giving the whole selection a single
    /// well-defined exit for post-dominance to be computed against.
    terms: Vec<Terminator>
}

impl Cfg {
    fn exit(&self) -> usize {
        self.blocks.len()
    }

    fn successors(&self, block: usize) -> Vec<usize> {
        match self.terms[block] {
            Terminator::Fallthrough(t) | Terminator::Jump(t) => vec![t],
            Terminator::Branch {
                taken, not_taken, ..
            } => vec![taken, not_taken]
        }
    }
}

fn mnemonic_of<T: ListingElement>(token: &T) -> Option<Mnemonic> {
    token.mnemonic().copied()
}

/// The target label name and, for a conditional jump, confirmation that a
/// flag condition is present - `Some(condition_present, label)`. For a
/// conditional jump (`JR cc,label`), `arg1` carries the flag test and
/// `arg2` the target; for an unconditional one (`JR label`), the parser
/// still places the sole argument in `arg2`, leaving `arg1` empty (verified
/// against the real parser output, not assumed). Fully generic via
/// `DataAccessElem`/`ExprElement`'s trait-level accessors - no matching on
/// concrete `DataAccess`/`Expr` variants, so this works identically for
/// `Token` and `LocatedToken`.
fn jump_condition_and_target<T: ListingElement>(token: &T) -> Option<(bool, &str)> {
    let arg1 = token.mnemonic_arg1();
    let arg2 = token.mnemonic_arg2();
    let (conditional, target_arg) = match (arg1, arg2) {
        (Some(a1), Some(_)) if a1.is_flag_test() => (true, arg2),
        (None, Some(_)) => (false, arg2),
        (Some(_), None) => (false, arg1),
        _ => return None
    };
    let label = target_arg?
        .get_expression()
        .filter(|e| e.is_label())
        .map(|e| e.label())?;
    Some((conditional, label))
}

/// Builds the control-flow graph for `tokens`, or a human-readable reason
/// it can't (loop, escaping target, or an unsupported mnemonic anywhere).
fn build_cfg<T: ListingElement>(tokens: &[T]) -> Result<Cfg, String> {
    // Pass 1: every label defined in `tokens` -> its index.
    let mut label_indices: HashMap<&str, usize> = HashMap::new();
    for (i, token) in tokens.iter().enumerate() {
        if token.is_label() {
            label_indices.insert(token.label_symbol(), i);
        }
    }

    // Pass 2: block boundaries - a new block starts at 0, at every label,
    // and right after every JR/JP/DJNZ/CALL/RET token (RET is a terminator
    // in every form even though it's no longer in UNSUPPORTED_MNEMONICS -
    // see the dedicated handling below).
    let mut block_starts = vec![0usize];
    for (i, token) in tokens.iter().enumerate() {
        if token.is_label() {
            block_starts.push(i);
        }
        if let Some(m) = mnemonic_of(token)
            && (JUMP_MNEMONICS.contains(&m)
                || UNSUPPORTED_MNEMONICS.contains(&m)
                || m == Mnemonic::Ret)
            && i + 1 < tokens.len()
        {
            block_starts.push(i + 1);
        }
    }
    block_starts.sort_unstable();
    block_starts.dedup();

    let blocks: Vec<BasicBlock> = block_starts
        .iter()
        .enumerate()
        .map(|(idx, &start)| {
            let end = block_starts
                .get(idx + 1)
                .map(|&n| n - 1)
                .unwrap_or(tokens.len() - 1);
            BasicBlock { start, end }
        })
        .collect();
    let index_to_block: HashMap<usize, usize> = blocks
        .iter()
        .enumerate()
        .map(|(idx, b)| (b.start, idx))
        .collect();
    let exit = blocks.len();

    let resolve_target = |label: &str, from_index: usize| -> Result<usize, String> {
        let target_index = *label_indices
            .get(label)
            .ok_or_else(|| format!("jump target \"{label}\" is not defined in the selection"))?;
        if target_index <= from_index {
            return Err(format!(
                "backward jump to \"{label}\" (a loop) isn't supported"
            ));
        }
        blocks
            .iter()
            .enumerate()
            .find(|(_, b)| (b.start..=b.end).contains(&target_index))
            .map(|(idx, _)| idx)
            .ok_or_else(|| format!("could not resolve jump target \"{label}\""))
    };

    let mut terms = Vec::with_capacity(blocks.len());
    for block in &blocks {
        let next_block = index_to_block
            .get(&(block.end + 1))
            .copied()
            .unwrap_or(exit);

        let last = &tokens[block.end];
        let Some(mnemonic) = mnemonic_of(last)
        else {
            terms.push(Terminator::Fallthrough(next_block));
            continue;
        };

        // RET is handled before the UNSUPPORTED_MNEMONICS check (it's no
        // longer in that list): its taken side - conditional or not -
        // leaves straight to the selection's own virtual exit, never to a
        // real label, so it needs no `resolve_target` call at all. See the
        // module doc comment for why this "cheat" gives the right answer.
        if mnemonic == Mnemonic::Ret {
            let conditional = last.mnemonic_arg1().is_some_and(|a| a.is_flag_test());
            if conditional {
                terms.push(Terminator::Branch {
                    taken: exit,
                    not_taken: next_block,
                    cost_taken: 0,
                    cost_not_taken: 0
                });
            }
            else {
                terms.push(Terminator::Jump(exit));
            }
            continue;
        }

        if UNSUPPORTED_MNEMONICS.contains(&mnemonic) {
            return Err(format!(
                "{mnemonic:?} at token index {} isn't supported (only JR/JP/RET are balanced)",
                block.end
            ));
        }
        if !JUMP_MNEMONICS.contains(&mnemonic) {
            terms.push(Terminator::Fallthrough(next_block));
            continue;
        }

        let Some((conditional, label)) = jump_condition_and_target(last)
        else {
            return Err(format!(
                "could not parse the {mnemonic:?} target at token index {}",
                block.end
            ));
        };
        let target = resolve_target(label, block.end)?;

        if !conditional {
            terms.push(Terminator::Jump(target));
            continue;
        }

        // Callers supply real taken/not-taken costs via `cost` in
        // `balance_branches` - here we only need to know *that* this is a
        // conditional jump to shape the CFG; the actual numbers are filled
        // in during `balance`, where the cost function is in scope.
        terms.push(Terminator::Branch {
            taken: target,
            not_taken: next_block,
            cost_taken: 0,
            cost_not_taken: 0
        });
    }

    Ok(Cfg { blocks, terms })
}

/// Immediate post-dominator of every block (index `cfg.exit()` for the
/// virtual exit itself). Exploits that every edge in this CFG goes
/// strictly forward in index order (loops were already rejected in
/// `build_cfg`) - blocks are therefore already in a valid reverse
/// postorder, so a single backward pass suffices; no fixpoint iteration is
/// needed the way a general CFG's dominance computation would require.
fn compute_postdominators(cfg: &Cfg) -> Vec<usize> {
    let exit = cfg.exit();
    let mut postdom = vec![exit; exit + 1];
    postdom[exit] = exit;

    fn intersect(postdom: &[usize], mut a: usize, mut b: usize) -> usize {
        while a != b {
            while a < b {
                a = postdom[a];
            }
            while b < a {
                b = postdom[b];
            }
        }
        a
    }

    for i in (0..cfg.blocks.len()).rev() {
        let succs = cfg.successors(i);
        let mut acc = succs[0];
        for &s in &succs[1..] {
            acc = intersect(&postdom, acc, s);
        }
        postdom[i] = acc;
    }
    postdom
}

/// Sums costs for `tokens[start..=end]` using each instruction's single
/// (`Fixed`) cost, or the *taken* cost for anything still `Conditional` at
/// this point - by the time this is called every branch inside the range
/// has already been resolved to a fixed cost by an earlier, innermost
/// step, so nothing left in range should still be genuinely dual-valued;
/// this is a defensive fallback, not expected to matter in practice.
/// Label tokens contribute nothing (a real, zero-cost marker).
fn straight_line_cost<T: ListingElement>(
    tokens: &[T],
    start: usize,
    end: usize,
    cost: &impl Fn(&T) -> InstructionCost
) -> Result<u32, String> {
    if start > end {
        return Ok(0);
    }
    let mut total = 0u32;
    for token in &tokens[start..=end] {
        if token.is_label() {
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
    tokens: &[T],
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
        } = cfg.terms[b]
        else {
            unreachable!()
        };
        // Real taken/not-taken cost of the branch instruction itself,
        // supplied by the caller's cost function (see `build_cfg`'s own
        // note on why this is deferred to here).
        let branch_token = &tokens[cfg.blocks[b].end];
        let (cost_taken, cost_not_taken) = match cost(branch_token) {
            InstructionCost::Conditional { taken, not_taken } => (taken, not_taken),
            InstructionCost::Fixed(n) => (n, n),
            InstructionCost::Unknown => {
                return Err("the branch instruction itself has no known cost".to_string());
            }
        };
        let merge = postdom[b];

        let taken_cost =
            cost_taken + arm_cost(cfg, &resolved, postdom, tokens, cost, taken, merge)?;
        let not_taken_cost =
            cost_not_taken + arm_cost(cfg, &resolved, postdom, tokens, cost, not_taken, merge)?;

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
            // `resolve_target`) - that's the one shape with no
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
    tokens: &[T],
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
            total += straight_line_cost(tokens, block.start, block.end, cost)?;
            cur = match cfg.terms[cur] {
                Terminator::Fallthrough(t) | Terminator::Jump(t) => t,
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
/// this v1 doesn't support (a loop, `DJNZ`/`CALL` in any form, or an
/// escaping jump target) - the caller is expected to offer no action at
/// all in that case, not surface the message as an error (this isn't
/// necessarily a mistake, just a shape this feature doesn't cover yet).
pub fn balance_branches<T: ListingElement>(
    tokens: &[T],
    cost: impl Fn(&T) -> InstructionCost
) -> Result<Vec<StabilizeEdit>, String> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    let cfg = build_cfg(tokens)?;
    if !cfg
        .terms
        .iter()
        .any(|t| matches!(t, Terminator::Branch { .. }))
    {
        return Ok(Vec::new());
    }
    let postdom = compute_postdominators(&cfg);
    let mut edits = balance(&cfg, &postdom, tokens, &cost)?;
    edits.sort_unstable_by(|a, b| b.anchor_index().cmp(&a.anchor_index()));
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::ListingElement;

    use super::*;
    use crate::parser::obtained::LocatedToken;
    use crate::parser::parse_z80_str;

    /// A tiny, test-only cost source mirroring the real Z80/CPC "NOPs"
    /// timing convention (1 NOP = 4 T-states) this whole feature is built
    /// around: `jr cc`/`ret cc` = 3 taken / 2 not-taken, unconditional
    /// `jr`/`ret` = 3, `ld r,r'`/`nop` = 1 each - the same real values used
    /// to hand-verify the shipped LSP version's own tests, kept identical
    /// here so the same hand-verified expected numbers still apply.
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
            Some(Mnemonic::Ld) | Some(Mnemonic::Nop) => InstructionCost::Fixed(1),
            Some(Mnemonic::Djnz) | Some(Mnemonic::Call) => InstructionCost::Unknown,
            _ => InstructionCost::Fixed(0)
        }
    }

    fn balance(code: &str) -> Result<Vec<StabilizeEdit>, String> {
        // `LocatedToken::clone()` is an `unimplemented!()` stub in this
        // codebase today, so tests borrow straight from the parsed
        // `LocatedListing` (which derefs down to `&[LocatedToken]`) rather
        // than collecting an owned `Vec`.
        let listing = parse_z80_str(code).unwrap();
        let tokens: &[LocatedToken] = &listing;
        balance_branches(tokens, test_cost)
    }

    /// The classic shape: `jr nz,.b` / cheap not-taken arm (ends with its
    /// own unconditional jump over) / `.b:` expensive taken arm / `.over:`.
    /// Hand-verified: taken path = 3 (jr taken) + 1 (ld a,b) + 1 (ld c,d) =
    /// 5; not-taken path = 2 (jr not taken) + 1 (ld a,b) + 3 (jr .over) =
    /// 6. The taken arm is cheaper by 1, so 1 NOP must land right before
    /// the token starting the `.over:` block (index 6: `jr nz,.b`(0),
    /// `ld a,b`(1), `jr .over`(2), `.b`(3), `ld a,b`(4), `ld c,d`(5),
    /// `.over`(6)).
    #[test]
    fn single_branch_pads_the_cheaper_arm() {
        let code = "    jr nz,.b\n    ld a,b\n    jr .over\n.b\n    ld a,b\n    ld c,d\n.over\n";
        let edits = balance(code).unwrap();
        assert_eq!(
            edits,
            vec![StabilizeEdit::InsertPadding {
                insert_before_index: 6,
                nop_count: 1
            }]
        );
    }

    #[test]
    fn already_balanced_branch_needs_no_edits() {
        // not-taken: 2 (jr) + 1+1 (nop*2) + 3 (jr .over) = 7
        // taken:      3 (jr) + 1+1+1+1 (nop*4) = 7
        let code = "    jr nz,.b\n    nop\n    nop\n    jr .over\n.b\n    nop\n    nop\n    nop\n    nop\n.over\n";
        assert!(balance(code).unwrap().is_empty());
    }

    #[test]
    fn nested_branch_is_resolved_innermost_first() {
        let code = "\
    jr nz,.outer_b
    nop
    jr .outer_over
.outer_b
    jr z,.inner_b
    nop
    jr .inner_over
.inner_b
    nop
    nop
.inner_over
.outer_over
";
        let edits = balance(code).unwrap();
        // Inner branch: taken (3) + nop*2 (2) = 5; not-taken (2) + nop (1)
        // + jr .inner_over (3) = 6 -> inner taken arm padded by 1, landing
        // right before the `.inner_over` token (index 10).
        assert!(
            edits.iter().any(|e| {
                matches!(
                    e,
                    StabilizeEdit::InsertPadding {
                        insert_before_index: 10,
                        nop_count: 1
                    }
                )
            }),
            "{edits:?}"
        );
        // Whatever the outer branch's own imbalance resolves to, it must
        // land at or before `.outer_over` (index 11), never past it.
        assert!(
            edits.iter().all(|e| {
                match e {
                    StabilizeEdit::InsertPadding {
                        insert_before_index,
                        ..
                    } => *insert_before_index <= 11,
                    StabilizeEdit::RewriteConditionalRetAndPad {
                        ret_token_index, ..
                    } => *ret_token_index <= 11
                }
            }),
            "{edits:?}"
        );
    }

    #[test]
    fn sibling_branches_are_each_balanced_independently() {
        let code = "\
    jr nz,.a_b
    nop
    jr .a_over
.a_b
    nop
    nop
.a_over
    jr z,.c_b
    nop
    jr .c_over
.c_b
    nop
    nop
    nop
.c_over
";
        let edits = balance(code).unwrap();
        // First branch: taken (3+2=5) vs not-taken (2+1+3=6) -> pad taken
        // arm by 1 before `.a_over` (index 6).
        assert!(
            edits.contains(&StabilizeEdit::InsertPadding {
                insert_before_index: 6,
                nop_count: 1
            }),
            "{edits:?}"
        );
        // Second branch: taken (3+3=6) vs not-taken (2+1+3=6) -> already
        // balanced, no edit for it.
        assert_eq!(edits.len(), 1, "{edits:?}");
    }

    #[test]
    fn a_backward_jump_is_rejected_as_a_loop() {
        let code = ".loop\n    nop\n    jr nz,.loop\n";
        assert!(balance(code).is_err());
    }

    #[test]
    fn djnz_and_call_are_rejected() {
        for instr in ["djnz .x", "call nz,.x", "call .x"] {
            let code = format!("    {instr}\n.x\n    nop\n");
            assert!(balance(&code).is_err(), "{instr}");
        }
    }

    /// A real-world idiom (from the user's own `bc26_hl` example): `RET cc`
    /// is no longer blanket-rejected the way `DJNZ`/`CALL` still are. A
    /// lone unconditional `RET` in particular - how nearly every real
    /// subroutine selection ends - must not abort the whole pass just
    /// because it's present (this was the actual root cause of the Quick
    /// Fix never appearing at all for real selections).
    #[test]
    fn a_lone_unconditional_ret_is_not_rejected() {
        let code = "    ld a,b\n    ret\n";
        assert!(balance(code).unwrap().is_empty());
    }

    /// `RET cc` alone, nothing following it in the selection: its taken
    /// side reaches the virtual exit directly (cost 3, no arm content -
    /// there is nothing after this token at all), and so does its
    /// not-taken side (cost 2, also no arm content - `next_block` defaults
    /// to `exit` when nothing follows). The not-taken side is cheaper by
    /// 1, and - unlike the taken side - it has a perfectly well-defined
    /// insertion point (right after this one token, index 1) via
    /// `arm_padding_index`'s own "end == exit" fallback, so this is a
    /// plain `InsertPadding`, not a rewrite.
    #[test]
    fn ret_cc_alone_pads_the_fallthrough_arm_via_plain_insertion() {
        let code = "    ret nc\n";
        let edits = balance(code).unwrap();
        assert_eq!(
            edits,
            vec![StabilizeEdit::InsertPadding {
                insert_before_index: 1,
                nop_count: 1
            }]
        );
    }

    /// The user's own idiom, minimally reproduced: `ret nc` (early exit)
    /// followed by more code ending in its own unconditional `ret`. Hand
    /// verified: taken (early-exit) path = 3 (ret nc taken) + 0 (nothing
    /// else - it leaves immediately) = 3; not-taken path = 2 (ret nc not
    /// taken) + 1 (nop) + 3 (the trailing ret) = 6. The early-exit arm is
    /// cheaper by 3, and it has no in-selection code to pad into - this
    /// must come back as a rewrite, not a plain insertion.
    #[test]
    fn ret_cc_pads_the_early_exit_arm_via_rewrite() {
        let code = "    ret nc\n    nop\n    ret\n";
        let edits = balance(code).unwrap();
        assert_eq!(
            edits,
            vec![StabilizeEdit::RewriteConditionalRetAndPad {
                ret_token_index: 0,
                nop_count: 3
            }]
        );
    }

    /// Same shape as the rewrite case above, but the fallthrough arm's own
    /// content (just one `nop`, no trailing `ret` - it simply runs off the
    /// end of the selection, itself reaching the virtual exit) is sized so
    /// both paths cost exactly 3: taken = 3 (ret nc taken); not-taken = 2
    /// (ret nc not taken) + 1 (nop) = 3.
    #[test]
    fn ret_cc_already_balanced_needs_no_edits() {
        let code = "    ret nc\n    nop\n";
        assert!(balance(code).unwrap().is_empty());
    }

    #[test]
    fn a_jump_target_outside_the_tokens_is_rejected() {
        let code = "    jr nz,.elsewhere\n    nop\n";
        assert!(balance(code).is_err());
    }

    #[test]
    fn a_selection_with_no_branch_at_all_yields_no_edits() {
        let code = "    ld a,b\n    ld c,d\n    nop\n";
        assert!(balance(code).unwrap().is_empty());
    }
}
