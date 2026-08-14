//! Is this register (or flag) still used after this point?
//!
//! A forward walk over the [`AnalysisStream`], answering the question
//! `regsNotUsedAfter`/`flagsNotUsedAfter` ask. Ported from upstream's
//! `Pattern.java::depUsedAfter`, whose shape matters and is preserved here:
//!
//! * **A worklist, not a simple recursion.** Each item is a
//!   `(position, dependency, call stack)` triple. A successor is only enqueued
//!   if no item already seen at that position had *both* the same remaining
//!   dependency and the same call stack - which is what makes loops terminate
//!   without missing a read inside one.
//! * **Three answers, not two.** "Definitely used", "definitely not used", and
//!   "can't tell". The last is folded into the first by every caller: an
//!   optimization that cannot be *proven* safe is not offered.
//! * **Calls are followed**, with a stack. `CALL` pushes its return address,
//!   `RET` pops it and continues after the call site. Treating either as an
//!   opaque barrier would be far weaker - real code calls constantly - while
//!   losing track of the stack (recursion, a `RET` with nothing to return to,
//!   anything doing arithmetic on `SP`) falls back to "can't tell".
//!
//! Every "can't tell" path below is a deliberate refusal to guess, not an
//! oversight: reaching data as if it were code, an unresolvable jump target,
//! `JP (HL)`, or simply running out of instructions all mean the walk no
//! longer knows what executes next.

use std::collections::HashMap;

use cpclib_tokens::{DataAccessElem, ListingElement, Mnemonic};

use crate::flow::{jump, successors};

use crate::analysis_op::{AnalysisOp, OpClass};
use crate::dependency::Dependency;
use crate::effects::effects_of;
use crate::regflag::Reg;
use crate::stream::AnalysisStream;

/// The answer to "is this dependency used after that point?", together with
/// the instruction that settled it.
///
/// The witness is what lets a suggestion explain itself. "Remove unused
/// `ld b, c`" is unauditable on its own - the user cannot tell whether B was
/// clobbered two instructions later or inside a routine three calls deep - so
/// the walk records the instruction that made it safe and hands it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Liveness {
    pub usage: Usage,
    /// For [`Usage::NotUsed`]: an example op that overwrote the dependency
    /// before anything read it. `None` when nothing overwrote it and every
    /// path simply ran out - a different and weaker reason, worth wording
    /// differently.
    ///
    /// Only ever *an* example: several paths can each kill it in their own
    /// place, and naming one is far more useful than naming none.
    pub witness: Option<usize>
}

impl Liveness {
    fn of(usage: Usage) -> Self {
        Self {
            usage,
            witness: None
        }
    }
}

/// The answer to "is this dependency used after that point?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    /// Something reachable really does read it.
    Used,
    /// Nothing reachable reads it before it is overwritten or execution
    /// provably ends.
    NotUsed,
    /// The walk lost track. Callers must treat this exactly like [`Usage::Used`]
    /// - it is "not proven safe", not "probably fine".
    Unknown
}

/// How many worklist items to expand before giving up.
///
/// A whole-file walk that follows calls can legitimately visit a lot of
/// states, but an LSP runs this per rule per match per keystroke, so an
/// unbounded walk is a real hang risk on pathological input. Hitting the cap
/// yields [`Usage::Unknown`], i.e. declines the optimization - the same answer
/// as any other loss of certainty.
const MAX_STATES: usize = 20_000;

/// One node of the search: where we are, what is still live, and how we got
/// here.
#[derive(Debug, Clone)]
struct State {
    position: usize,
    dependency: Dependency,
    /// Return addresses of the calls currently entered. `None` means the
    /// stack is no longer trustworthy (something modified `SP` in a way this
    /// analysis does not model), so any `RET` from here is unknowable.
    call_stack: Option<Vec<usize>>
}

/// Walk forward from `start` and report whether `dependency` is ever read.
///
/// `labels` maps a label name to the op index it marks, and is what makes
/// jumps followable; a jump to a name absent from it is unresolvable and ends
/// the walk in [`Usage::Unknown`].
pub fn is_used_after<T>(
    stream: &AnalysisStream<'_, T>,
    labels: &HashMap<String, usize>,
    start: usize,
    dependency: Dependency
) -> Liveness
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    // The stack pointer is live everywhere, whatever the visible code does
    // with it. On a CPC an interrupt can fire between any two instructions and
    // pushes the return address onto the stack, so SP is read by code that
    // appears nowhere in the listing - and the firmware runs on interrupts by
    // default. Walking forward can only ever see the *visible* reads, so it
    // will happily report SP as dead in code like
    //
    // ```text
    //     ld sp, $
    // frame:
    //     jp frame        ; nothing here reads SP...
    // ```
    //
    // which is a real sequence from `birthtro`, and deleting that `ld sp`
    // leaves interrupts pushing over whatever the stack pointer happened to
    // hold.
    if dependency == Dependency::Reg(Reg::Sp) {
        return Liveness::of(Usage::Used);
    }

    let ops = stream.ops();
    let mut worklist = vec![State {
        position: start,
        dependency,
        call_stack: Some(Vec::new())
    }];
    // Every `(position, dependency, call stack)` already enqueued. Keyed by
    // position, holding every distinct (dependency, stack) seen there - the
    // same shape upstream uses, and the reason a loop whose body narrows the
    // dependency differently is still explored rather than skipped.
    let mut seen: HashMap<usize, Vec<(Dependency, Option<Vec<usize>>)>> = HashMap::new();
    let mut expanded = 0usize;
    let mut witness: Option<usize> = None;

    while let Some(state) = worklist.pop() {
        expanded += 1;
        if expanded > MAX_STATES {
            return Liveness::of(Usage::Unknown);
        }

        let Some(op) = ops.get(state.position)
        else {
            // Ran off the end of what we can see. The stream is the whole
            // flattened file, so this really is "we don't know what runs
            // next", not "the program stops".
            return Liveness::of(Usage::Unknown);
        };

        // Anything this analysis cannot account for ends the walk. Stepping
        // over it would assume it touches no register, which is exactly the
        // assumption that is unsafe to make: data reached as if it were code
        // means the walk has lost the thread, and basm's multi-register
        // `push bc, hl` does not parse to a plain opcode, so it arrives with
        // no mnemonic at all. Treating that as inert made a sprite loop's
        // `ld b, height` look dead - the `push bc` reading B had simply been
        // stepped over - and the optimizer offered to delete it.
        //
        // See [`OpClass`] for the classification itself, which is shared with
        // the block-local constraints so the two can never drift apart.
        let executes = match op.classify() {
            OpClass::Inert => false,
            OpClass::Opaque => return Liveness::of(Usage::Unknown),
            OpClass::Executes => true
        };

        let mut dependency = state.dependency;

        if executes {
            let Some(effects) = effects_of(op)
            else {
                // No table row - an instruction whose behavior we cannot
                // describe. Never assume it touches nothing.
                return Liveness::of(Usage::Unknown);
            };

            let reads_it = effects
                .reads
                .iter()
                .any(|r| dependency.matches(Dependency::Reg(*r)))
                || effects
                    .reads_flags
                    .iter()
                    .any(|f| dependency.matches(Dependency::Flag(*f)));
            if reads_it {
                return Liveness::of(Usage::Used);
            }

            // Not read here - now apply what this instruction overwrites,
            // narrowing the dependency as each part of it is clobbered.
            let mut alive = Some(dependency);
            for written in &effects.writes {
                let Some(current) = alive
                else {
                    break;
                };
                alive = current.after_write(Dependency::Reg(*written));
            }
            for written in &effects.writes_flags {
                let Some(current) = alive
                else {
                    break;
                };
                alive = current.after_write(Dependency::Flag(*written));
            }

            let Some(remaining) = alive
            else {
                // Fully overwritten. Nothing further along *this* path can
                // reveal a use, so abandon it - without concluding anything
                // about the other paths still in the worklist.
                //
                // This is the instruction that makes the optimization safe, so
                // remember the first one seen: it is what a suggestion points
                // at when asked why.
                witness.get_or_insert(state.position);
                continue;
            };
            dependency = remaining;
        }

        // Where can execution go from here?
        let successors = match successors_of(ops, labels, state.position, &state.call_stack) {
            Some(next) => next,
            None => return Liveness::of(Usage::Unknown)
        };

        for (position, call_stack) in successors {
            let entry = seen.entry(position).or_default();
            if entry
                .iter()
                .any(|(d, s)| *d == dependency && *s == call_stack)
            {
                continue;
            }
            entry.push((dependency, call_stack.clone()));
            worklist.push(State {
                position,
                dependency,
                call_stack
            });
        }
    }

    Liveness {
        usage: Usage::NotUsed,
        witness
    }
}

/// Every place execution can continue from `position`, with the call stack it
/// would have there. `None` means "cannot tell".
fn successors_of<T>(
    ops: &[AnalysisOp<'_, T>],
    labels: &HashMap<String, usize>,
    position: usize,
    call_stack: &Option<Vec<usize>>
) -> Option<Vec<(usize, Option<Vec<usize>>)>>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let op = ops.get(position)?;
    let fallthrough = position + 1;
    let mnemonic = op.mnemonic();

    // Two things this view cares about that are *not* control-flow shape, so
    // they stay here rather than in the shared table:
    //
    // * `HALT` stops the CPU until an interrupt fires, and the handler can
    //   clobber anything. Continuing past it would be a fail-open. (The timing
    //   view has no such concern: a `halt` costs what it costs and control
    //   does continue afterwards.)
    // * `PUSH`/`POP` shift what a later `RET` would find, and this analysis
    //   does not track *what* is pushed, so it cannot trust the stack after
    //   one.
    if mnemonic == Some(Mnemonic::Halt) {
        return None;
    }

    let arg1 = op.arg1();
    let arg2 = op.arg2();
    let edges = successors::edges_of(
        mnemonic,
        successors::Policy::DATAFLOW,
        jump::is_conditional(arg1.as_deref()),
        || {
            if mnemonic == Some(Mnemonic::Djnz) {
                return jump::djnz_target(arg1.as_deref())
                    .and_then(|name| labels.get(name).copied())
                    .map(|target| (true, target));
            }
            let (conditional, target) = jump::condition_and_target(arg1.as_deref(), arg2.as_deref())?;
            let name = jump::label_of(target)?;
            Some((conditional, labels.get(name).copied()?))
        }
    );

    match edges {
        successors::Edges::Fallthrough => {
            let stack = match mnemonic {
                Some(Mnemonic::Push | Mnemonic::Pop) => None,
                _ => call_stack.clone()
            };
            Some(vec![(fallthrough, stack)])
        },

        successors::Edges::Jump(target) => Some(vec![(target, call_stack.clone())]),

        successors::Edges::Branch(target) => {
            Some(vec![
                (target, call_stack.clone()),
                (fallthrough, call_stack.clone()),
            ])
        },

        successors::Edges::Call { target, conditional } => {
            let mut stack = call_stack.clone()?;
            stack.push(fallthrough);
            let mut next = vec![(target, Some(stack))];
            if conditional {
                next.push((fallthrough, call_stack.clone()));
            }
            Some(next)
        },

        // `Policy::DATAFLOW` returns to whoever called - so this walk has to
        // have seen the call. Returning to a caller it never entered leaves
        // the continuation genuinely unknown.
        successors::Edges::Return { conditional } => {
            let stack = call_stack.as_ref()?;
            let (&target, rest) = stack.split_last()?;
            let mut next = vec![(target, Some(rest.to_vec()))];
            if conditional {
                next.push((fallthrough, call_stack.clone()));
            }
            Some(next)
        },

        successors::Edges::Unknown => None
    }
}

/// Index every label in the stream, so jumps can be followed.
pub fn label_index<T>(stream: &AnalysisStream<'_, T>) -> HashMap<String, usize>
where T: ListingElement {
    let mut labels = HashMap::new();
    for (index, op) in stream.ops().iter().enumerate() {
        if op.is_label() {
            labels
                .entry(op.origin().label_symbol().to_string())
                .or_insert(index);
        }
    }
    labels
}
