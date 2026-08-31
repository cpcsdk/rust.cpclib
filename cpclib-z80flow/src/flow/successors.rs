//! What control flow does after one instruction — the decision table both
//! views share, and the one place their disagreements are written down.
//!
//! ## What is shared, and what deliberately is not
//!
//! The *operand plumbing* is [`super::jump`]'s job. The *graph construction*
//! stays with each view, because they genuinely build different things: [`cfg`]
//! groups tokens into basic blocks and resolves labels to block indices, while
//! [`liveness`] works per-op over an expanded stream and resolves labels to op
//! indices. Neither is a worse version of the other.
//!
//! What *was* duplicated is this table: given a mnemonic, is there a
//! fallthrough, is there a transfer, is it conditional, and does a `CALL` or a
//! `RET` mean anything special. Both copies had to agree about `DJNZ` always
//! being conditional, about `RET cc` having a fallthrough, and about `RST`
//! being opaque — and nothing made them agree except that one person wrote
//! both.
//!
//! ## The disagreements are real, so they are a [`Policy`], not a bug
//!
//! A timing query and a register-liveness query want different things from the
//! same `CALL`, and both are right:
//!
//! * **Timing** asks "what does the path through *this selection* cost". A
//!   `call` is one instruction with a fixed cost; the callee is somewhere else.
//!   (Phase 3 lets it price the callee too — but by *asking* for that cost, not
//!   by turning the call into a graph edge, which would wreck the
//!   post-dominator computation `branch_balance` relies on.)
//! * **Liveness** asks "what can execute after this point". Skipping into the
//!   callee would miss every register the callee clobbers, so it follows the
//!   call with a real call stack and pops it on `RET`.
//!
//! [`cfg`]: crate::cfg
//! [`liveness`]: crate::liveness

use cpclib_tokens::Mnemonic;

/// How a view treats `CALL` (and `RST`, which follows it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallPolicy {
    /// A call is an ordinary instruction: control continues after it, and the
    /// callee is not part of this graph. What the timing views use.
    StraightLine,
    /// A call transfers into the callee, remembering where to come back to.
    /// What liveness uses, because a callee's register writes are exactly what
    /// it must not miss.
    Follow
}

/// How a view treats `RET`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReturnPolicy {
    /// The return leaves this analysis's view entirely and reaches its single
    /// virtual exit. Every path through a selection converges there whichever
    /// `RET` fires, which is the shape both timing consumers want.
    Exit,
    /// The return goes back to whoever called — the caller pops its own stack
    /// to find out where, and fails closed if it never saw the call.
    PopCallStack
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Policy {
    pub(crate) call: CallPolicy,
    pub(crate) ret: ReturnPolicy
}

impl Policy {
    /// The timing view: calls are straight-line, returns reach the exit.
    pub(crate) const TIMING: Self = Self {
        call: CallPolicy::StraightLine,
        ret: ReturnPolicy::Exit
    };

    /// The dataflow view: calls are followed, returns pop the call stack.
    pub(crate) const DATAFLOW: Self = Self {
        call: CallPolicy::Follow,
        ret: ReturnPolicy::PopCallStack
    };
}

/// Where control goes after one instruction.
///
/// Generic in `X`, the caller's own notion of a resolved target: [`cfg`] uses
/// its `Successor` (which already encodes forward-block / back-edge / escapes),
/// [`liveness`] uses a plain op index. That is what lets one table serve both
/// without either having to adopt the other's idea of what a target is.
///
/// [`cfg`]: crate::cfg
/// [`liveness`]: crate::liveness
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Edges<X> {
    /// Control simply continues at the next position.
    Fallthrough,
    /// Unconditional transfer; nothing falls through.
    Jump(X),
    /// Two-way: `X` when taken, the next position otherwise.
    Branch(X),
    /// A followed call: transfer to `X`, and the next position is where the
    /// matching `RET` should come back to. `conditional` adds a fallthrough.
    Call { target: X, conditional: bool },
    /// A return; where it goes is the caller's business per [`ReturnPolicy`].
    Return { conditional: bool },
    /// Nothing here can be resolved. Every consumer treats this as "control
    /// flow is unknown from here" and fails closed rather than guessing.
    Unknown
}

/// Does control go anywhere other than the next instruction?
///
/// The same table, asked a question that needs no target resolution - which is
/// exactly what deciding a basic block's boundary needs. Keeping this derived
/// from [`edges_of`] rather than as its own list of mnemonics is the point:
/// `cfg` used to carry `JUMP_MNEMONICS.contains(&m) || m == Ret || m == Djnz`,
/// which had already drifted (it missed `JQ`, `RETI` and `RETN`) and would
/// drift again the next time the table learned an instruction.
pub(crate) fn transfers_control(mnemonic: Option<Mnemonic>, policy: Policy, conditional: bool) -> bool {
    // `resolve` answering `None` turns a real transfer into `Unknown`, which
    // is still "not a fallthrough" - and no label lookup is paid for.
    !matches!(
        edges_of(mnemonic, policy, conditional, || None::<(bool, ())>),
        Edges::Fallthrough
    )
}

/// The table itself.
///
/// `resolve` is called at most once, and only when the mnemonic actually has a
/// target worth resolving — so a caller pays for label lookup only where one is
/// needed. It returns `(is_conditional, target)`, or `None` when the operand
/// names nothing this view can resolve.
pub(crate) fn edges_of<X>(
    mnemonic: Option<Mnemonic>,
    policy: Policy,
    conditional: bool,
    resolve: impl FnOnce() -> Option<(bool, X)>
) -> Edges<X> {
    let Some(mnemonic) = mnemonic
    else {
        // A label, an `org`, a comment: nothing transfers.
        return Edges::Fallthrough;
    };

    match mnemonic {
        Mnemonic::Ret | Mnemonic::Reti | Mnemonic::Retn => {
            // `conditional` comes from the caller, not from `resolve`: a
            // `RET cc`'s flag test is its *only* operand, so the target decoder
            // reads it as an unconditional one-operand form. Only the plain
            // `RET` has a conditional form at all.
            Edges::Return {
                conditional: conditional && mnemonic == Mnemonic::Ret
            }
        },

        Mnemonic::Call | Mnemonic::Rst => {
            match policy.call {
                // The callee is not part of this graph; the call instruction
                // itself is just another instruction control passes through.
                CallPolicy::StraightLine => Edges::Fallthrough,
                CallPolicy::Follow => {
                    // `RST` jumps into firmware no analysis here has a view of,
                    // so following it is not possible even in principle.
                    if mnemonic == Mnemonic::Rst {
                        return Edges::Unknown;
                    }
                    match resolve() {
                        Some((conditional, target)) => {
                            Edges::Call {
                                target,
                                conditional
                            }
                        },
                        None => Edges::Unknown
                    }
                }
            }
        },

        Mnemonic::Jp | Mnemonic::Jr | Mnemonic::Jq => {
            match resolve() {
                Some((true, target)) => Edges::Branch(target),
                Some((false, target)) => Edges::Jump(target),
                None => Edges::Unknown
            }
        },

        // Always two-way: loop back, or fall through once `B` hits zero.
        Mnemonic::Djnz => {
            match resolve() {
                Some((_, target)) => Edges::Branch(target),
                None => Edges::Unknown
            }
        },

        _ => Edges::Fallthrough
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `resolve` must not run for an instruction that has no target - callers
    /// rely on that to avoid a label lookup per ordinary instruction.
    #[test]
    fn an_ordinary_instruction_never_asks_for_a_target() {
        let mut asked = false;
        let edges = edges_of(Some(Mnemonic::Nop), Policy::TIMING, false, || {
            asked = true;
            None::<(bool, usize)>
        });
        assert_eq!(edges, Edges::Fallthrough);
        assert!(!asked, "a nop has no target to resolve");
    }

    /// The one difference that motivated the whole policy split.
    #[test]
    fn a_call_is_straight_line_for_timing_and_followed_for_dataflow() {
        assert_eq!(
            edges_of(Some(Mnemonic::Call), Policy::TIMING, false, || Some((false, 7usize))),
            Edges::Fallthrough,
            "timing prices the call instruction, not the callee's graph"
        );
        assert_eq!(
            edges_of(Some(Mnemonic::Call), Policy::DATAFLOW, false, || Some((false, 7usize))),
            Edges::Call {
                target: 7,
                conditional: false
            },
            "liveness must see what the callee clobbers"
        );
    }

    /// `RST` lands in firmware. Following it is not possible, and pretending
    /// otherwise would silently claim a routine clobbers nothing.
    #[test]
    fn rst_is_unknown_when_calls_are_followed() {
        assert_eq!(
            edges_of(Some(Mnemonic::Rst), Policy::DATAFLOW, false, || Some((false, 7usize))),
            Edges::Unknown
        );
        assert_eq!(
            edges_of(Some(Mnemonic::Rst), Policy::TIMING, false, || Some((false, 7usize))),
            Edges::Fallthrough
        );
    }

    /// `DJNZ` is conditional whatever its operand decoding says - there is no
    /// flag test to read, and it always has a fallthrough.
    #[test]
    fn djnz_is_always_two_way() {
        assert_eq!(
            edges_of(Some(Mnemonic::Djnz), Policy::TIMING, false, || Some((false, 3usize))),
            Edges::Branch(3)
        );
    }

    /// An unresolvable target is never silently dropped into a fallthrough:
    /// every consumer has to see that flow became unknown here.
    #[test]
    fn an_unresolvable_transfer_is_unknown_not_fallthrough() {
        for mnemonic in [Mnemonic::Jp, Mnemonic::Jr, Mnemonic::Djnz] {
            assert_eq!(
                edges_of(Some(mnemonic), Policy::TIMING, false, || None::<(bool, usize)>),
                Edges::Unknown,
                "{mnemonic:?}"
            );
        }
    }

    /// `RET cc` is conditional through the dedicated flag, never through the
    /// target decoder - the regression that made this argument exist.
    #[test]
    fn a_conditional_return_is_conditional_without_having_a_target() {
        assert_eq!(
            edges_of(Some(Mnemonic::Ret), Policy::TIMING, true, || None::<(bool, usize)>),
            Edges::Return { conditional: true }
        );
        assert_eq!(
            edges_of(Some(Mnemonic::Ret), Policy::TIMING, false, || None::<(bool, usize)>),
            Edges::Return { conditional: false }
        );
        assert_eq!(
            edges_of(Some(Mnemonic::Reti), Policy::TIMING, true, || None::<(bool, usize)>),
            Edges::Return { conditional: false },
            "reti has no conditional form"
        );
    }

    /// A label or directive carries no mnemonic and simply continues.
    #[test]
    fn a_position_with_no_mnemonic_falls_through() {
        assert_eq!(
            edges_of(None, Policy::DATAFLOW, false, || None::<(bool, usize)>),
            Edges::Fallthrough
        );
    }
}
