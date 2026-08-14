//! Control-flow-graph construction over a token slice - shared by
//! `branch_balance` (timing-balancing edits) and `cost_range` (min/max cost
//! queries), the two very different *consumers* of "what does the control
//! flow of this selection look like."
//!
//! Deliberately **permissive**: unlike an earlier version of this logic
//! (which lived inside `branch_balance` and hard-`Err`ed the instant it saw
//! a loop, an escaping jump target, `DJNZ`, or `CALL`), `build_cfg` here
//! always succeeds structurally - a backward jump becomes a real
//! `Successor::Loop`, an unresolvable target becomes `Successor::Escapes`,
//! `DJNZ` is modeled as a genuine loop-continuation branch, and `CALL` isn't
//! a terminator at all (its own cost just folds into whichever block it
//! falls inside - a real subroutine call's own instruction has a
//! well-defined, fixed cost independent of the callee's body). Whether any
//! of that is actually *acceptable* is entirely up to each consumer:
//! `branch_balance::balance_branches` validates the resulting `Cfg` and
//! rejects exactly the same shapes it always has (see `validate_forward_only`);
//! `cost_range::cost_range` accepts `Loop`/`Escapes` directly and degrades
//! gracefully (unbounded max with a real min, or "this path just ends
//! here"). Building one permissive structure once, with each consumer
//! interpreting it their own way, avoids maintaining two parallel CFG
//! builders that would otherwise drift apart.
//!
//! `RET` (conditional or not) keeps the "cheat to the virtual exit" model
//! `branch_balance` originally established: a real routine's early return
//! genuinely does leave to an unrelated call site, but every path through
//! the selection converges there regardless of which `RET` fires, so
//! treating "taken" as reaching the shared exit directly - never a real
//! label - gives the right shape for both consumers without either needing
//! to know anything about the caller.

use std::collections::HashMap;

use cpclib_tokens::{ListingElement, Mnemonic};

use crate::flow::{jump, successors};

pub(crate) fn mnemonic_of<T: ListingElement>(token: &T) -> Option<Mnemonic> {
    token.mnemonic().copied()
}

/// What a jump/branch/loop instruction's own target resolves to.
#[derive(Debug, Clone)]
pub(crate) enum Successor {
    /// A real, forward, in-selection block.
    Block(usize),
    /// The target resolves to a label at or before the jump's own index -
    /// a back-edge. Carries the label text purely for error-message
    /// quality (`branch_balance`'s validation reproduces its original
    /// message text exactly).
    Loop { label: String },
    /// The target label isn't defined anywhere in the given tokens.
    Escapes { label: String }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasicBlock {
    pub(crate) start: usize,
    pub(crate) end: usize // inclusive
}

pub(crate) enum Terminator {
    Fallthrough(usize),
    Jump(Successor),
    /// Two-way. Carries no costs: an earlier version had `cost_taken` /
    /// `cost_not_taken` fields here, but they were only ever constructed as
    /// `0` and `branch_balance` computed its own from the caller's cost
    /// function anyway. Pricing belongs where the cost function is in scope,
    /// not on the graph.
    Branch {
        taken: Successor,
        not_taken: usize
    }
}

pub(crate) struct Cfg {
    pub(crate) blocks: Vec<BasicBlock>,
    /// One terminator per real block; index `blocks.len()` is a virtual
    /// exit node every terminator target that falls off the end of the
    /// tokens resolves to, giving the whole selection a single
    /// well-defined exit for post-dominance/cost-range purposes.
    pub(crate) terms: Vec<Terminator>
}

/// A `Successor` already confirmed to be `Block` by prior validation - only
/// ever called on a `Cfg` `branch_balance` has already run
/// `validate_forward_only` against.
pub(crate) fn expect_block(s: &Successor) -> usize {
    match s {
        Successor::Block(idx) => *idx,
        _ => unreachable!("Cfg must be validated (no Loop/Escapes) before this is called")
    }
}

impl Cfg {
    pub(crate) fn exit(&self) -> usize {
        self.blocks.len()
    }

    /// Successors as plain, resolved block indices - panics if any
    /// successor is a `Loop`/`Escapes` marker. Only valid once the caller
    /// has already run `validate_forward_only`; `cost_range` never calls
    /// this, it walks `Successor` directly instead.
    pub(crate) fn successors_resolved(&self, block: usize) -> Vec<usize> {
        match &self.terms[block] {
            Terminator::Fallthrough(t) => vec![*t],
            Terminator::Jump(s) => vec![expect_block(s)],
            Terminator::Branch {
                taken, not_taken, ..
            } => vec![expect_block(taken), *not_taken]
        }
    }

    /// `Err` with a human-readable reason (reproducing this module's
    /// pre-refactor error text exactly) the first time any terminator's
    /// target is a `Successor::Loop`/`Escapes` - `Ok(())` once every jump
    /// target is confirmed to be a real, forward, in-selection block.
    /// `branch_balance::balance_branches` calls this right after
    /// `build_cfg`, since balancing needs a fully resolved CFG;
    /// `cost_range` never calls this at all.
    pub(crate) fn validate_forward_only(&self) -> Result<(), String> {
        for term in &self.terms {
            let maybe_successor = match term {
                Terminator::Jump(s) => Some(s),
                Terminator::Branch { taken, .. } => Some(taken),
                Terminator::Fallthrough(_) => None
            };
            if let Some(s) = maybe_successor {
                match s {
                    Successor::Loop { label } => {
                        return Err(format!(
                            "backward jump to \"{label}\" (a loop) isn't supported"
                        ));
                    },
                    Successor::Escapes { label } => {
                        return Err(format!(
                            "jump target \"{label}\" is not defined in the selection"
                        ));
                    },
                    Successor::Block(_) => {}
                }
            }
        }
        Ok(())
    }
}

/// The target label name and, for a conditional jump, confirmation that a
/// flag condition is present - `Some(condition_present, label)`.
///
/// Which operand slot the target lives in is [`flow::jump`]'s business, shared
/// with the optimizer's own walk - see there for the parser quirk involved.
fn jump_condition_and_target<T: ListingElement>(token: &T) -> Option<(bool, &str)> {
    let (conditional, target) =
        jump::condition_and_target(token.mnemonic_arg1(), token.mnemonic_arg2())?;
    Some((conditional, jump::label_of(target)?))
}

/// `DJNZ`'s own sole operand is always its loop target - no flag test to
/// distinguish, unlike `JR`/`RET`.
fn djnz_target<T: ListingElement>(token: &T) -> Option<&str> {
    jump::djnz_target(token.mnemonic_arg1())
}

/// Every label defined in `tokens` -> its index.
///
/// A local (dot-prefixed) label is registered under *both* its bare form and
/// its fully-qualified form (`<nearest preceding global>.local`) - mirroring
/// basm's own symbol-table resolution (`set_current_global_label`/
/// `extend_local_and_patterns_for_symbol`, `cpclib-tokens/src/symbols/
/// table.rs`). A reference can legitimately be written either way: bare,
/// relying on basm's own ambient "current global" tracking (the common case
/// for hand-written code), or explicitly qualified (what `stabilize.rs`'s own
/// rewritten `JR` targets always are, per the user's own correction that an
/// unqualified local reference isn't reliable). Without this, analyzing a
/// selection containing stabilize's own generated output - a qualified
/// reference pointing at a bare-defined local label - spuriously "escapes"
/// even though the label is right there.
///
/// Shared with `cost_range`, which resolves `CALL` targets against the same
/// index rather than growing a second, subtly-different copy of these rules.
pub(crate) fn label_indices<T: ListingElement>(tokens: &[&T]) -> HashMap<String, usize> {
    let mut label_indices: HashMap<String, usize> = HashMap::new();
    let mut current_global: Option<&str> = None;
    for (i, token) in tokens.iter().enumerate() {
        if token.is_label() {
            let name = token.label_symbol();
            if let Some(local) = name.strip_prefix('.') {
                if let Some(global) = current_global {
                    label_indices.insert(format!("{global}.{local}"), i);
                }
            }
            else {
                current_global = Some(name);
            }
            label_indices.insert(name.to_string(), i);
        }
    }
    label_indices
}

/// Builds the control-flow graph for `tokens`. Always succeeds
/// structurally (see the module doc comment) except for a genuine
/// parse-shape anomaly this function itself can't interpret at all (e.g. a
/// `JR`/`JP`/`DJNZ` whose operand isn't recognizable as a label at all) -
/// that remains a real `Err`, not a policy choice either consumer could
/// reasonably override.
pub(crate) fn build_cfg<T: ListingElement>(tokens: &[&T]) -> Result<Cfg, String> {
    let label_indices = label_indices(tokens);

    // Pass 2: block boundaries - a new block starts at 0, at every label,
    // and right after every JR/JP/RET/DJNZ token (a real terminator in
    // every form). CALL is deliberately *not* included - see the module
    // doc comment for why it isn't a terminator here at all.
    let mut block_starts = vec![0usize];
    for (i, token) in tokens.iter().enumerate() {
        if token.is_label() {
            block_starts.push(i);
        }
        // Asked of the shared table rather than of a list kept here: a block
        // ends wherever control can go somewhere other than the next
        // instruction. `CALL` deliberately does not (see the module doc
        // comment), and `Policy::TIMING` is what says so.
        if successors::transfers_control(
            mnemonic_of(*token),
            successors::Policy::TIMING,
            jump::is_conditional(token.mnemonic_arg1())
        ) && i + 1 < tokens.len()
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

    let resolve_successor = |label: &str, from_index: usize| -> Successor {
        let Some(&target_index) = label_indices.get(label)
        else {
            return Successor::Escapes {
                label: label.to_string()
            };
        };
        if target_index <= from_index {
            return Successor::Loop {
                label: label.to_string()
            };
        }
        blocks
            .iter()
            .position(|b| (b.start..=b.end).contains(&target_index))
            .map(Successor::Block)
            .unwrap_or_else(|| {
                Successor::Escapes {
                    label: label.to_string()
                }
            })
    };

    let mut terms = Vec::with_capacity(blocks.len());
    for block in &blocks {
        let next_block = index_to_block
            .get(&(block.end + 1))
            .copied()
            .unwrap_or(exit);

        let last = tokens[block.end];

        // One shared decision table decides *what shape* the flow out of this
        // block is (see `flow::successors`); resolving a label to a real block
        // stays here, because "which block" is this view's own idea of a
        // target and no other view shares it.
        let mnemonic = mnemonic_of(last);
        let edges = successors::edges_of(
            mnemonic,
            successors::Policy::TIMING,
            jump::is_conditional(last.mnemonic_arg1()),
            || {
            if mnemonic == Some(Mnemonic::Djnz) {
                // `DJNZ`'s sole operand is its target, never a flag test; the
                // table already knows it is unconditional-in-form but two-way
                // in fact, so the flag it gets here is irrelevant.
                return djnz_target(last).map(|label| (true, resolve_successor(label, block.end)));
            }
                jump_condition_and_target(last).map(|(conditional, label)| {
                    (conditional, resolve_successor(label, block.end))
                })
            }
        );

        terms.push(match edges {
            successors::Edges::Fallthrough => Terminator::Fallthrough(next_block),
            successors::Edges::Jump(target) => Terminator::Jump(target),
            successors::Edges::Branch(target) => {
                Terminator::Branch {
                    taken: target,
                    not_taken: next_block
                }
            },
            // `Policy::TIMING` never follows a call, so this is unreachable -
            // but expressing it as a fallthrough rather than a panic keeps the
            // one place that decides call semantics in `flow::successors`.
            successors::Edges::Call { .. } => Terminator::Fallthrough(next_block),
            // A `RET`'s taken side leaves straight to the selection's own
            // virtual exit, never to a real label - see the module doc comment
            // for why that "cheat" gives both consumers the right shape.
            successors::Edges::Return { conditional: true } => {
                Terminator::Branch {
                    taken: Successor::Block(exit),
                    not_taken: next_block
                }
            },
            successors::Edges::Return { conditional: false } => {
                Terminator::Jump(Successor::Block(exit))
            },
            successors::Edges::Unknown => {
                return Err(format!(
                    "could not parse the {} target at token index {}",
                    mnemonic
                        .map(|m| format!("{m:?}"))
                        .unwrap_or_else(|| "branch".to_owned()),
                    block.end
                ));
            }
        });
    }

    Ok(Cfg { blocks, terms })
}

/// Immediate post-dominator of every block (index `cfg.exit()` for the
/// virtual exit itself). Exploits that every edge in an already-validated
/// (forward-only) CFG goes strictly forward in index order - blocks are
/// therefore already in a valid reverse postorder, so a single backward
/// pass suffices; no fixpoint iteration is needed the way a general CFG's
/// dominance computation would require. Only ever called on a `Cfg` that
/// has already passed `validate_forward_only` (via `successors_resolved`,
/// which panics on an unresolved `Successor` otherwise).
pub(crate) fn compute_postdominators(cfg: &Cfg) -> Vec<usize> {
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
        let succs = cfg.successors_resolved(i);
        let mut acc = succs[0];
        for &s in &succs[1..] {
            acc = intersect(&postdom, acc, s);
        }
        postdom[i] = acc;
    }
    postdom
}
