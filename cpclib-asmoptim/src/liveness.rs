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

use cpclib_tokens::{DataAccessElem, ExprElement, ListingElement, Mnemonic};

use crate::analysis_op::AnalysisOp;
use crate::dependency::Dependency;
use crate::effects::effects_of;
use crate::regflag::Reg;
use crate::stream::AnalysisStream;

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
) -> Usage
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
        return Usage::Used;
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

    while let Some(state) = worklist.pop() {
        expanded += 1;
        if expanded > MAX_STATES {
            return Usage::Unknown;
        }

        let Some(op) = ops.get(state.position)
        else {
            // Ran off the end of what we can see. The stream is the whole
            // flattened file, so this really is "we don't know what runs
            // next", not "the program stops".
            return Usage::Unknown;
        };

        // Data reached as if it were code means either a bug or hand-placed
        // bytes; either way the analysis has lost the thread.
        if op.is_data() {
            return Usage::Unknown;
        }

        // Anything that is neither an instruction nor a label is something
        // this analysis cannot account for, and stepping over it would be
        // assuming it touches no register - which is exactly the assumption
        // that is unsafe to make.
        //
        // A label really is inert (it is a position, nothing executes), so it
        // is the one thing that may be skipped. Everything else that reaches
        // here does so *because* it was not understood: basm's multi-register
        // `push bc, hl`, for one, does not parse to a plain opcode, so it
        // arrives with no mnemonic at all. Treating that as inert made a
        // sprite loop's `ld b, height` look dead - the `push bc` that reads B
        // had simply been stepped over - and the optimizer offered to delete
        // it.
        if op.mnemonic().is_none() && !op.is_label() {
            return Usage::Unknown;
        }

        let mut dependency = state.dependency;

        if op.mnemonic().is_some() {
            let Some(effects) = effects_of(op)
            else {
                // No table row - an instruction whose behavior we cannot
                // describe. Never assume it touches nothing.
                return Usage::Unknown;
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
                return Usage::Used;
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
                continue;
            };
            dependency = remaining;
        }

        // Where can execution go from here?
        let successors = match successors_of(ops, labels, state.position, &state.call_stack) {
            Some(next) => next,
            None => return Usage::Unknown
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

    Usage::NotUsed
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

    let Some(mnemonic) = op.mnemonic()
    else {
        // Labels, `org`, ... - control simply continues.
        return Some(vec![(fallthrough, call_stack.clone())]);
    };

    match mnemonic {
        Mnemonic::Ret | Mnemonic::Reti | Mnemonic::Retn => {
            let conditional = op.arg1().is_some_and(|a| a.get_flag_test().is_some());
            let stack = call_stack.as_ref()?;
            let Some((&target, rest)) = stack.split_last()
            else {
                // Returning to a caller this walk never entered: the
                // continuation is genuinely unknown.
                return None;
            };
            let mut next = vec![(target, Some(rest.to_vec()))];
            if conditional {
                next.push((fallthrough, call_stack.clone()));
            }
            Some(next)
        },

        Mnemonic::Call | Mnemonic::Rst => {
            // `RST` jumps into firmware this analysis has no view of.
            if mnemonic == Mnemonic::Rst {
                return None;
            }
            let (conditional, target) = jump_target(op, labels)?;
            let mut stack = call_stack.clone()?;
            stack.push(fallthrough);
            let mut next = vec![(target, Some(stack))];
            if conditional {
                next.push((fallthrough, call_stack.clone()));
            }
            Some(next)
        },

        Mnemonic::Jp | Mnemonic::Jr => {
            let (conditional, target) = jump_target(op, labels)?;
            let mut next = vec![(target, call_stack.clone())];
            if conditional {
                next.push((fallthrough, call_stack.clone()));
            }
            Some(next)
        },

        Mnemonic::Djnz => {
            // Always conditional: loop back, or fall through when `B` hits
            // zero.
            let target = op
                .arg1()
                .and_then(|a| label_of(a.as_ref()))
                .and_then(|name| labels.get(&name).copied())?;
            Some(vec![
                (target, call_stack.clone()),
                (fallthrough, call_stack.clone()),
            ])
        },

        // Anything that leaves the CPU's control, or that this analysis
        // cannot follow.
        Mnemonic::Halt => None,

        _ => {
            // Ordinary instruction. Only the stack discipline needs care:
            // `PUSH`/`POP` shift what a later `RET` would find, and this
            // analysis does not track *what* is pushed, so it can no longer
            // trust the stack afterwards.
            let stack = match mnemonic {
                Mnemonic::Push | Mnemonic::Pop => None,
                _ => call_stack.clone()
            };
            Some(vec![(fallthrough, stack)])
        }
    }
}

/// `(is_conditional, target op index)` for a jump/call, or `None` when the
/// target isn't a resolvable label - a computed `JP (HL)`, an expression, or a
/// label defined outside this listing.
fn jump_target<T>(op: &AnalysisOp<'_, T>, labels: &HashMap<String, usize>) -> Option<(bool, usize)>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let arg1 = op.arg1();
    let arg2 = op.arg2();
    let (conditional, target) = match (&arg1, &arg2) {
        (Some(a1), Some(_)) if a1.get_flag_test().is_some() => (true, arg2.as_ref()),
        (None, Some(_)) => (false, arg2.as_ref()),
        (Some(_), None) => (false, arg1.as_ref()),
        _ => return None
    };
    let name = label_of(target?.as_ref())?;
    Some((conditional, labels.get(&name).copied()?))
}

/// The label an operand names, if it names one directly.
fn label_of(operand: &cpclib_tokens::DataAccess) -> Option<String> {
    operand
        .get_expression()
        .filter(|e| e.is_label())
        .map(|e| e.label().to_string())
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

#[cfg(test)]
mod tests {
    use cpclib_asm::flatten::flatten_for_analysis;
    use cpclib_asm::parser::{LocatedToken, parse_z80_str};

    use super::*;
    use crate::regflag::{Flag, Reg};
    use crate::stream::build_without_addresses;

    /// Walk from just after the instruction on 0-based `after_index` (counting
    /// only real instructions, so tests read like the source).
    fn usage(source: &str, after_index: usize, dep: Dependency) -> Usage {
        let listing = parse_z80_str(source).expect("source must parse");
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let stream = build_without_addresses(&tokens);
        let labels = label_index(&stream);

        let nth_instruction = stream
            .ops()
            .iter()
            .enumerate()
            .filter(|(_, op)| op.mnemonic().is_some())
            .map(|(i, _)| i)
            .nth(after_index)
            .expect("instruction index out of range");

        is_used_after(&stream, &labels, nth_instruction + 1, dep)
    }

    fn reg(r: Reg) -> Dependency {
        Dependency::Reg(r)
    }

    #[test]
    fn a_value_read_by_the_next_instruction_is_used() {
        // ld a, 1 / ld b, a  -> A is read.
        assert_eq!(
            usage("    ld a, 1\n    ld b, a\n    ret\n", 0, reg(Reg::A)),
            Usage::Used
        );
    }

    #[test]
    fn a_value_overwritten_before_any_read_is_not_used() {
        // ld a, 1 / ld a, 2 / ret -> the first A is dead.
        assert_eq!(
            usage("    ld a, 1\n    ld a, 2\n    ret\n", 0, reg(Reg::A)),
            Usage::NotUsed
        );
    }

    /// The narrowing model end to end: writing `B` leaves `C` live, so a
    /// later read of `C` still counts as using `BC`.
    #[test]
    fn writing_one_half_leaves_the_other_half_live() {
        assert_eq!(
            usage("    ld bc, 0\n    ld b, 1\n    ld a, c\n    ret\n", 0, reg(Reg::Bc)),
            Usage::Used
        );
        // ...and once *both* halves are rewritten, the original is dead.
        assert_eq!(
            usage(
                "    ld bc, 0\n    ld b, 1\n    ld c, 2\n    ld a, c\n    ret\n",
                0,
                reg(Reg::Bc)
            ),
            Usage::NotUsed
        );
    }

    /// A `RET` this walk never entered via a `CALL` goes somewhere unknown.
    #[test]
    fn returning_to_an_unknown_caller_is_unknown() {
        assert_eq!(
            usage("    ld a, 1\n    ret\n", 0, reg(Reg::A)),
            Usage::Unknown
        );
    }

    /// Falling off the end of the file is not "the program ended" - the
    /// stream is only what we can see.
    #[test]
    fn running_out_of_instructions_is_unknown() {
        assert_eq!(usage("    ld a, 1\n    nop\n", 0, reg(Reg::A)), Usage::Unknown);
    }

    /// The loop cases the memoized worklist exists for.
    #[test]
    fn a_loop_that_reads_the_value_reports_it_used() {
        let source = "\
    ld a, 1
loop:
    inc b
    or a
    jr nz, loop
    ret
";
        // `A` is read by `or a` inside the loop.
        assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
    }

    #[test]
    fn a_loop_that_never_touches_the_value_terminates_and_reports_not_used() {
        let source = "\
    ld a, 1
loop:
    inc b
    dec c
    jr nz, loop
    ld a, 2
    ret
";
        // Nothing in the loop reads `A`, and it is overwritten after it.
        assert_eq!(usage(source, 0, reg(Reg::A)), Usage::NotUsed);
    }

    /// Both arms of a conditional branch have to be explored - a read on
    /// either side counts.
    #[test]
    fn both_arms_of_a_branch_are_explored() {
        let source = "\
    ld a, 1
    jr z, taken
    ld a, 2
    jr done
taken:
    ld b, a
done:
    ld a, 3
    ret
";
        // The taken arm reads `A`; the fallthrough overwrites it.
        assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
    }

    /// A call is followed into and back out of.
    #[test]
    fn a_call_is_followed_and_returns_to_the_call_site() {
        // NB: the label can't be called `sub` - that's a real Z80 mnemonic.
        let source = "\
    ld a, 1
    call routine
    ld b, a
    ret
routine:
    nop
    ret
";
        // The subroutine doesn't touch `A`, but the instruction after the
        // call reads it - so the walk must come back.
        assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
    }

    #[test]
    fn a_value_read_inside_a_called_subroutine_is_used() {
        let source = "\
    ld a, 1
    call routine
    ret
routine:
    ld b, a
    ret
";
        assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
    }

    /// A computed jump has an unknowable continuation.
    #[test]
    fn a_computed_jump_is_unknown() {
        assert_eq!(
            usage("    ld a, 1\n    jp (hl)\n", 0, reg(Reg::A)),
            Usage::Unknown
        );
    }

    /// Reaching data as if it were code means the analysis lost the thread.
    #[test]
    fn falling_into_data_is_unknown() {
        assert_eq!(
            usage("    ld a, 1\n    defb 0, 1, 2\n", 0, reg(Reg::A)),
            Usage::Unknown
        );
    }

    /// Flags work exactly like registers, including being killed by an
    /// instruction that rewrites them.
    #[test]
    fn a_flag_overwritten_before_any_test_is_not_used() {
        assert_eq!(
            usage(
                "    or a\n    ld a, 5\n    cp 3\n    jr z, done\ndone:\n    ret\n",
                0,
                Dependency::Flag(Flag::Z)
            ),
            Usage::NotUsed
        );
    }

    #[test]
    fn a_flag_tested_by_a_later_branch_is_used() {
        assert_eq!(
            usage(
                "    or a\n    ld a, 5\n    jr z, done\ndone:\n    ret\n",
                0,
                Dependency::Flag(Flag::Z)
            ),
            Usage::Used
        );
    }
}
