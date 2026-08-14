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
    /// How many instructions the *selection* contains - not how many run.
    ///
    /// Deliberately unchanged by call-following: a callee's body is priced
    /// into `min`/`max`, but its instructions are not in the selection the
    /// user pointed at, and counting them would make this number stop
    /// describing what is on screen.
    pub instruction_count: u32,
    /// Recognized-as-an-instruction tokens whose cost source didn't know a
    /// timing for them - `min`/`max` are lower bounds when this is nonzero.
    pub unrecognized_count: u32,
    /// A `CALL` reached a routine defined outside the analysed tokens, so its
    /// body could not be priced - `min`/`max` count the call instruction
    /// itself and nothing more. Another way for the totals to be lower bounds,
    /// kept separate from `unrecognized_count` because the cause is different:
    /// nothing here is unrecognized, it is simply not in view.
    pub incomplete: bool
}

/// A straight-line run's cost, which stops being a single number as soon as
/// a `CALL` in it is priced by what the callee does.
#[derive(Debug, Clone, Copy, Default)]
struct RunCost {
    min: u32,
    max: u32,
    /// A callee somewhere in this run loops, or recurses - so `max` is a
    /// partial sum rather than a real upper bound.
    unbounded: bool,
    /// A callee could not be priced (it is defined outside this selection),
    /// so the totals are lower bounds.
    incomplete: bool
}

/// State threaded through a whole `cost_range` query so that pricing a callee
/// happens once, and so that recursion terminates.
///
/// **Keyed by the callee's token index in the outermost slice**, never the
/// slice a sub-query happens to be looking at. Each sub-query analyses
/// `&tokens[entry..]`, so its own indices restart at zero - keying on those
/// made two different routines collide on the same number and, worse, made
/// mutual recursion invisible, because `ping` was index 2 at the top level and
/// index 0 inside `pong`'s own query. `base` is what maps one to the other.
#[derive(Default)]
struct CallCosts {
    memo: std::collections::HashMap<usize, RunCost>,
    /// Callees currently being priced. A `CALL` reaching one of these is a
    /// cycle - direct or mutual recursion - whose total cost depends on a
    /// runtime iteration count no static pass can know.
    in_progress: std::collections::HashSet<usize>
}

/// What a `call` at `token` costs, including the callee's own body.
///
/// This is the one place the timing view looks past the instruction in front
/// of it. Note what it deliberately does *not* do: it does not add a
/// call/return edge to the graph. Doing so would make the CFG cyclic and break
/// `cfg::compute_postdominators`, which `branch_balance` depends on and which
/// is only correct because every edge runs forward. Pricing a callee is a
/// separate, memoised query; the call site stays straight-line.
fn call_cost<T: ListingElement>(
    outer: &[&T],
    labels: &std::collections::HashMap<String, usize>,
    token: &T,
    cost: &impl Fn(&T) -> InstructionCost,
    calls: &mut CallCosts
) -> RunCost {
    let own = match cost(token) {
        InstructionCost::Fixed(n) => (n, n),
        InstructionCost::Conditional { taken, not_taken } => (taken, not_taken),
        InstructionCost::Unknown => (0, 0)
    };
    let (taken, not_taken) = own;

    // Where does it go? An operand that is not a plain label - a computed
    // `call (hl)`-alike, or an expression - is not something to guess about.
    let target = crate::flow::jump::condition_and_target(
        token.mnemonic_arg1(),
        token.mnemonic_arg2()
    );
    let Some((conditional, target)) = target
    else {
        return RunCost {
            min: taken,
            max: taken,
            unbounded: false,
            incomplete: true
        };
    };
    // Resolved against the *outermost* slice's labels, so a routine defined
    // earlier in the file than its caller is still found. Resolving against
    // the sub-slice a nested query happens to be looking at would make every
    // backward call escape - and, worse, would hide mutual recursion, since
    // the second leg of `ping`/`pong` points back before `pong` begins.
    let Some(key) = crate::flow::jump::label_of(target).and_then(|name| labels.get(name).copied())
    else {
        // A callee defined outside the analysed tokens keeps the old
        // behaviour: the call instruction's own cost, and a note that the
        // total is a lower bound.
        return RunCost {
            min: taken,
            max: taken,
            unbounded: false,
            incomplete: true
        };
    };
    let body = if let Some(cached) = calls.memo.get(&key) {
        *cached
    }
    else if !calls.in_progress.insert(key) {
        // Recursion. The body's cost depends on how many times it recurses,
        // which is a runtime property - so the honest answer is "unbounded",
        // not a number that happens to be one level deep.
        RunCost {
            unbounded: true,
            ..RunCost::default()
        }
    }
    else {
        // The callee runs from its label to whichever `RET` it reaches - and
        // that is exactly what a `cost_range` starting at the label computes,
        // because `Policy::TIMING` sends every `RET` to the virtual exit.
        let body = match cost_range_inner(outer, key, labels, cost, calls) {
            Ok(range) => {
                RunCost {
                    min: range.min,
                    max: range.max,
                    unbounded: range.unbounded,
                    // Propagated, not reset: if the callee itself called
                    // something out of view, the caller's total is a lower
                    // bound too.
                    incomplete: range.incomplete
                }
            },
            // A callee this pass cannot interpret is not a reason to fail the
            // caller's whole query - it is one unpriceable body.
            Err(_) => {
                RunCost {
                    incomplete: true,
                    ..RunCost::default()
                }
            }
        };
        calls.in_progress.remove(&key);
        calls.memo.insert(key, body);
        body
    };

    if conditional {
        // `call cc` either happens (paying the callee) or does not. Exactly
        // like a conditional branch, the cheap side bounds `min` and the
        // expensive side bounds `max`.
        RunCost {
            min: not_taken.min(taken + body.min),
            max: not_taken.max(taken + body.max),
            unbounded: body.unbounded,
            incomplete: body.incomplete
        }
    }
    else {
        RunCost {
            min: taken + body.min,
            max: taken + body.max,
            unbounded: body.unbounded,
            incomplete: body.incomplete
        }
    }
}

/// The exact cost of a `call`, callee body included - or `None` when that is
/// not a single known number.
///
/// This is the narrow door `branch_balance` uses. Balancing *inserts padding*
/// to make two arms cost the same, so it can only work with an exact figure: a
/// callee whose own cost is a range (it branches), unbounded (it loops or
/// recurses), or incomplete (it is defined elsewhere) has no single answer, and
/// the balancer must decline rather than pad against a guess.
///
/// `cost_range` itself has no such restriction - reporting a range is its whole
/// job.
pub(crate) fn exact_call_cost<T: ListingElement>(
    tokens: &[&T],
    labels: &std::collections::HashMap<String, usize>,
    token: &T,
    cost: &impl Fn(&T) -> InstructionCost
) -> Option<u32> {
    let run = call_cost(tokens, labels, token, cost, &mut CallCosts::default());
    (!run.unbounded && !run.incomplete && run.min == run.max).then_some(run.min)
}

/// Sums `cost()` over `tokens[range]`, skipping labels, tracking
/// `instruction_count`/`unrecognized_count` as it goes. Uses a
/// `Conditional` cost's *taken* value as a defensive fallback (matches
/// `branch_balance::straight_line_cost`'s own convention) - by design,
/// this is only ever called on a range that doesn't include a block's own
/// diverging branch instruction (that one is priced separately, see
/// `cost_range`'s own `Terminator::Branch` handling).
///
/// A `CALL` is the one instruction in here whose cost is not just its own:
/// see [`call_cost`].
#[allow(clippy::too_many_arguments)]
fn sum_range<T: ListingElement>(
    outer: &[&T],
    tokens: &[&T],
    range: std::ops::Range<usize>,
    cost: &impl Fn(&T) -> InstructionCost,
    labels: &std::collections::HashMap<String, usize>,
    calls: &mut CallCosts,
    instruction_count: &mut u32,
    unrecognized_count: &mut u32
) -> RunCost {
    let mut total = RunCost::default();
    for token in tokens[range].iter().copied() {
        if token.is_label() {
            continue;
        }
        if token.mnemonic().copied() == Some(cpclib_tokens::Mnemonic::Call) {
            let call = call_cost(outer, labels, token, cost, calls);
            total.min += call.min;
            total.max += call.max;
            total.unbounded |= call.unbounded;
            total.incomplete |= call.incomplete;
            *instruction_count += 1;
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
                total.min += n;
                total.max += n;
                *instruction_count += 1;
            },
            InstructionCost::Conditional { taken, .. } => {
                total.min += taken;
                total.max += taken;
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
    let labels = crate::cfg::label_indices(tokens);
    cost_range_inner(tokens, 0, &labels, &cost, &mut CallCosts::default())
}

/// The cost of every path from `outer[base]` onward.
///
/// `outer` is always the *whole* set of tokens the outermost query was given,
/// and `base` says where this particular query starts within it - a callee's
/// label, for the recursive sub-queries [`call_cost`] makes. Keeping the full
/// slice rather than handing down `&tokens[base..]` is what lets `labels`
/// (built once, from `outer`) stay meaningful at every depth: a call to a
/// routine defined *earlier* than its caller still resolves, and the memo keys
/// mean the same thing everywhere.
fn cost_range_inner<T: ListingElement>(
    outer: &[&T],
    base: usize,
    labels: &std::collections::HashMap<String, usize>,
    cost: &impl Fn(&T) -> InstructionCost,
    calls: &mut CallCosts
) -> Result<CostRange, String> {
    let tokens = &outer[base.min(outer.len())..];
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
    let mut incomplete = false;

    for i in (0..cfg.blocks.len()).rev() {
        let block = cfg.blocks[i];
        match &cfg.terms[i] {
            Terminator::Fallthrough(next) => {
                let own = sum_range(
                    outer,
                    tokens,
                    block.start..block.end + 1,
                    cost,
                    &labels,
                    calls,
                    &mut instruction_count,
                    &mut unrecognized_count
                );
                incomplete |= own.incomplete;
                best_min[i] = own.min + best_min[*next];
                best_max[i] = own.max + best_max[*next];
                block_unbounded[i] = own.unbounded || block_unbounded[*next];
            },
            Terminator::Jump(successor) => {
                let own = sum_range(
                    outer,
                    tokens,
                    block.start..block.end + 1,
                    cost,
                    &labels,
                    calls,
                    &mut instruction_count,
                    &mut unrecognized_count
                );
                incomplete |= own.incomplete;
                match successor {
                    Successor::Block(next) => {
                        best_min[i] = own.min + best_min[*next];
                        best_max[i] = own.max + best_max[*next];
                        block_unbounded[i] = own.unbounded || block_unbounded[*next];
                    },
                    Successor::Loop { .. } => {
                        // An unconditional backward jump never reaches
                        // this selection's own exit via this path at all
                        // (as far as a static pass can tell) - only this
                        // block's own visible cost is well-defined.
                        best_min[i] = own.min;
                        best_max[i] = own.max;
                        block_unbounded[i] = true;
                    },
                    Successor::Escapes { .. } => {
                        best_min[i] = own.min;
                        best_max[i] = own.max;
                        block_unbounded[i] = own.unbounded;
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
                    outer,
                    tokens,
                    block.start..block.end,
                    cost,
                    &labels,
                    calls,
                    &mut instruction_count,
                    &mut unrecognized_count
                );
                incomplete |= prefix.incomplete;
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
                best_min[i] = prefix.min + min;
                best_max[i] = prefix.max + max;
                block_unbounded[i] = prefix.unbounded || unbounded;
            }
        }
    }

    Ok(CostRange {
        min: best_min[0],
        max: best_max[0],
        unbounded: block_unbounded[0],
        instruction_count,
        unrecognized_count,
        incomplete
    })
}
