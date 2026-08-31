//! Control flow and instruction semantics over a Z80 token slice.
//!
//! Everything here answers some form of "what does this sequence of
//! instructions actually do, and in what order does it do it" - and nothing
//! here knows anything about assembling, parsing, source spans or files. The
//! only dependency is `cpclib-tokens`, so every function is generic over
//! `ListingElement` and works identically on a plain `Token` and on the
//! parser's `LocatedToken`.
//!
//! ## Why this crate exists
//!
//! Two separate consumers had each grown their own model of the same thing:
//!
//! * `cpclib-asm` carried [`cfg`], [`cost_range`] and [`branch_balance`] - the
//!   NOP-cost and NOP-padding analyses behind the editor's cycle counter and
//!   its branch balancer. None of it used the assembler, and nothing in the
//!   assembler used it; it was 1400 lines of token analysis sitting in a crate
//!   that is already the parser, the assembler, the disassembler and more.
//! * `cpclib-asmoptim` carried [`analysis_op`], [`stream`], [`effects`],
//!   [`dependency`], [`liveness`] and [`regflag`] - the register/flag dataflow
//!   behind the peephole optimizer's safety constraints.
//!
//! Both walked the same instructions, decoded the same jump operands, and had
//! independently rediscovered the same parser quirks doing it. They now share
//! one successor relation ([`flow`]) and one place where that knowledge lives.
//!
//! ## The two views
//!
//! The consumers genuinely disagree about what a `CALL` or a back-edge means,
//! and those disagreements are *deliberate* - see [`flow::Policy`], which makes
//! each one an explicit, documented choice rather than a difference between two
//! bodies of code. [`cfg`] is a block-forming layer over the relation;
//! [`liveness`] is a reachability layer over the same relation.

pub mod analysis_op;
pub mod branch_balance;
pub(crate) mod cfg;
pub mod cost;
pub mod cost_range;
pub mod dependency;
pub mod effects;
pub(crate) mod flow;
pub mod liveness;
pub mod regflag;
pub mod stream;

pub use branch_balance::{StabilizeEdit, balance_branches};
pub use cost::{CostModel, InstructionCost};
pub use cost_range::{CostRange, cost_range};

/// Does control go anywhere other than the next instruction?
///
/// For a caller that walks straight-line code and wants to stop the moment it
/// no longer can - a jump, a call, a return, a loop back. Answers from the same
/// table [`cfg`] and [`liveness`] are built on, so a scan using it cannot fall
/// out of step with them the way a hand-written list of mnemonics does.
///
/// `HALT` is deliberately *not* a divert: control does continue after the
/// interrupt that wakes it. A caller that cares about what an interrupt
/// handler clobbers has to say so itself - see `liveness`, which does.
pub fn diverts_control(mnemonic: Option<cpclib_tokens::Mnemonic>) -> bool {
    // `conditional` only distinguishes `RET` from `RET cc`, and both divert.
    flow::successors::transfers_control(mnemonic, flow::successors::Policy::DATAFLOW, false)
}
