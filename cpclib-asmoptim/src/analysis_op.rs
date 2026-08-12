//! The instruction stream the analyzer actually reasons about.
//!
//! A parsed listing is *not* directly analyzable: it contains basm's "fake
//! instructions" (`ld hl, de`, `srl8 hl`, ... - convenience forms that
//! assemble to several real opcodes), `JQ` (which picks `JP` or `JR` at
//! assembly time depending on how far the target is), and plenty of tokens
//! that aren't instructions at all. Reasoning about registers and flags means
//! reasoning about what the CPU really executes, so the stream is normalized
//! once, up front - the same thing mdlz80optimizer does at parse time
//! (`CPUOpParser.java`'s `fakeInstructionEquivalent` expansion).
//!
//! [`AnalysisOp`] is that normalized view. It is deliberately a sum type
//! rather than a struct carrying an "is this real?" flag: the distinction is
//! resolved once, here, and every consumer downstream just reads
//! [`AnalysisOp::mnemonic`]/[`AnalysisOp::arg1`]/... without ever asking
//! whether it's looking at something the user literally typed.

use std::borrow::Cow;

use cpclib_tokens::{DataAccess, DataAccessElem, ListingElement, Mnemonic, Register8, Token};

/// One real instruction (or one non-instruction token) in the analysis
/// stream.
///
/// `T` is only ever the *original* token type (`LocatedToken` in the LSP,
/// `Token` in tests); it is carried purely so [`AnalysisOp::origin`] can hand
/// a diagnostic or a quickfix back the exact token the user wrote.
#[derive(Debug)]
pub enum AnalysisOp<'t, T> {
    /// A real instruction, exactly as the user wrote it.
    Real(&'t T),

    /// One real instruction standing in for something the user wrote that
    /// isn't directly executable - a fake instruction's expansion, or a `JQ`
    /// resolved to the concrete `JP`/`JR` it assembles to.
    Expanded {
        /// The token this came from. Every step of one expansion shares it.
        origin: &'t T,
        /// 0-based position of this step within its expansion...
        step: usize,
        /// ...out of this many. Together, `step` and `total` are what let a
        /// quickfix distinguish "this match covers the whole original
        /// instruction" (safe to rewrite, or to re-emit verbatim if the
        /// replacement left it alone) from "this match cuts one in half"
        /// (must decline - half an expansion can't be written back).
        total: usize,
        /// The real instruction this step executes.
        ///
        /// A whole [`Token`] rather than loose `(mnemonic, arg1, arg2)`
        /// fields: `Token::OpCode` carries a *fourth* `Option<Register8>`
        /// operand (the undocumented `SLA (IX+d), B` family) that loose
        /// fields would silently drop, and a whole token stays correct if the
        /// shape ever grows again. `Token: ListingElement` too, so the
        /// accessors below are literally the same calls for both variants.
        op: Token
    },

    /// A token that isn't an instruction: a label (a jump target), a data
    /// directive (`db`/`dw`), `org`, `equ`, ...
    ///
    /// Kept in the stream rather than filtered out, because a forward walk
    /// genuinely needs them - labels to resolve jump targets, and data to
    /// fail closed if execution ever reaches it (data reached as code means
    /// either a bug or hand-assembled bytes; upstream treats it as "assume
    /// the dependency is used", `Pattern.java`).
    Other(&'t T)
}

impl<'t, T> AnalysisOp<'t, T>
where T: ListingElement
{
    /// The token the user actually wrote. Several consecutive ops can share
    /// one origin (an expansion), so this is *not* unique across the stream.
    pub fn origin(&self) -> &'t T {
        match self {
            Self::Real(t) | Self::Other(t) => t,
            Self::Expanded { origin, .. } => origin
        }
    }

    /// Whether this op stands in for something the user wrote in a different
    /// form (a fake instruction, or a resolved `JQ`).
    pub fn is_expanded(&self) -> bool {
        matches!(self, Self::Expanded { .. })
    }

    /// `(step, total)` for an expanded op - see [`AnalysisOp::Expanded`]'s
    /// fields. `None` for anything the user wrote directly.
    pub fn expansion_position(&self) -> Option<(usize, usize)> {
        match self {
            Self::Expanded { step, total, .. } => Some((*step, *total)),
            _ => None
        }
    }

    /// This instruction's mnemonic, or `None` if it isn't an instruction.
    pub fn mnemonic(&self) -> Option<Mnemonic> {
        match self {
            Self::Real(t) => t.mnemonic().copied(),
            Self::Expanded { op, .. } => op.mnemonic().copied(),
            Self::Other(_) => None
        }
    }

    /// First operand, normalized to a plain [`DataAccess`].
    ///
    /// Borrowed whenever the underlying token already stores a plain
    /// `DataAccess` (so the common case allocates nothing); owned only when
    /// the token's own operand type has to be converted.
    pub fn arg1(&self) -> Option<Cow<'_, DataAccess>> {
        match self {
            Self::Real(t) => t.mnemonic_arg1().map(DataAccessElem::to_data_access),
            Self::Expanded { op, .. } => op.mnemonic_arg1().map(DataAccessElem::to_data_access),
            Self::Other(_) => None
        }
    }

    /// Second operand - see [`AnalysisOp::arg1`].
    pub fn arg2(&self) -> Option<Cow<'_, DataAccess>> {
        match self {
            Self::Real(t) => t.mnemonic_arg2().map(DataAccessElem::to_data_access),
            Self::Expanded { op, .. } => op.mnemonic_arg2().map(DataAccessElem::to_data_access),
            Self::Other(_) => None
        }
    }

    /// The undocumented third operand, when there is one - see
    /// `ListingElement::mnemonic_arg3`. Rare, but an analysis that ignored it
    /// would think `SLA (IX+d), B` never touches `B`.
    pub fn arg3(&self) -> Option<Register8> {
        match self {
            Self::Real(t) => t.mnemonic_arg3(),
            Self::Expanded { op, .. } => op.mnemonic_arg3(),
            Self::Other(_) => None
        }
    }

    /// Whether this is a label definition - i.e. a possible jump target.
    pub fn is_label(&self) -> bool {
        match self {
            Self::Other(t) => t.is_label(),
            _ => false
        }
    }

    /// Whether this is a comment.
    ///
    /// Inert like a label, and worth its own accessor because the alternative
    /// is treating it as "carries no mnemonic, therefore unanalysable" - which
    /// would make a single comment inside a matched region defeat every
    /// constraint asking what that region does, in code that is full of them.
    pub fn is_comment(&self) -> bool {
        match self {
            Self::Other(t) => t.is_comment(),
            _ => false
        }
    }

    /// Whether this emits raw data (`db`/`dw`). Reaching one while walking
    /// execution flow means the analysis has lost the thread and must fail
    /// closed.
    pub fn is_data(&self) -> bool {
        match self {
            Self::Other(t) => t.is_db() || t.is_dw(),
            _ => false
        }
    }
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::{DataAccess, Mnemonic, Register8, Register16, Token};

    use super::*;

    fn opcode(
        mnemonic: Mnemonic,
        arg1: Option<DataAccess>,
        arg2: Option<DataAccess>,
        arg3: Option<Register8>
    ) -> Token {
        Token::OpCode(mnemonic, arg1, arg2, arg3)
    }

    /// The whole point of the type: a `Real` op answers exactly what the
    /// underlying token answers, so nothing downstream needs to know which
    /// variant it holds.
    #[test]
    fn a_real_op_delegates_every_accessor_to_its_token() {
        let token = opcode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Register8(Register8::B)),
            None
        );
        let op = AnalysisOp::Real(&token);

        assert_eq!(op.mnemonic(), Some(Mnemonic::Ld));
        assert_eq!(op.arg1().as_deref(), Some(&DataAccess::Register8(Register8::A)));
        assert_eq!(op.arg2().as_deref(), Some(&DataAccess::Register8(Register8::B)));
        assert_eq!(op.arg3(), None);
        assert!(!op.is_expanded());
        assert_eq!(op.expansion_position(), None);
        assert!(std::ptr::eq(op.origin(), &token));
    }

    /// An `Expanded` op answers from its own stored instruction, not from
    /// `origin` - `origin` is only ever the *source location* handle.
    #[test]
    fn an_expanded_op_answers_from_its_own_instruction_not_its_origin() {
        // The user wrote a fake `ld hl, de`; this step is a real `ex de, hl`.
        let origin = opcode(
            Mnemonic::Ld,
            Some(DataAccess::Register16(Register16::Hl)),
            Some(DataAccess::Register16(Register16::De)),
            None
        );
        let op = AnalysisOp::Expanded {
            origin: &origin,
            step: 0,
            total: 3,
            op: opcode(Mnemonic::ExHlDe, None, None, None)
        };

        assert_eq!(op.mnemonic(), Some(Mnemonic::ExHlDe));
        assert_eq!(op.arg1(), None);
        assert!(op.is_expanded());
        assert_eq!(op.expansion_position(), Some((0, 3)));
        // ...while still pointing back at what the user actually typed.
        assert!(std::ptr::eq(op.origin(), &origin));
        assert_eq!(op.origin().mnemonic(), Some(&Mnemonic::Ld));
    }

    /// The accessor that exists precisely because `ListingElement` gained
    /// `mnemonic_arg3` - without it this instruction looks like it never
    /// touches `B`.
    #[test]
    fn the_undocumented_third_operand_is_visible_on_both_variants() {
        let token = opcode(
            Mnemonic::Sla,
            Some(DataAccess::IndexRegister16WithIndex(
                cpclib_tokens::IndexRegister16::Ix,
                cpclib_tokens::BinaryOperation::Add,
                0.into()
            )),
            None,
            Some(Register8::B)
        );
        assert_eq!(AnalysisOp::<Token>::Real(&token).arg3(), Some(Register8::B));

        let other = opcode(Mnemonic::Nop, None, None, None);
        let expanded = AnalysisOp::Expanded {
            origin: &other,
            step: 0,
            total: 1,
            op: token.clone()
        };
        assert_eq!(expanded.arg3(), Some(Register8::B));
    }

    #[test]
    fn a_non_instruction_token_reports_no_instruction_data() {
        let label = Token::Label("start".into());
        let op = AnalysisOp::Other(&label);

        assert_eq!(op.mnemonic(), None);
        assert_eq!(op.arg1(), None);
        assert_eq!(op.arg2(), None);
        assert_eq!(op.arg3(), None);
        assert!(op.is_label());
        assert!(!op.is_data());
        assert!(!op.is_expanded());
    }

    #[test]
    fn data_directives_are_recognised_so_a_walk_can_fail_closed_on_them() {
        let db = Token::Defb(vec![0.into()]);
        assert!(AnalysisOp::<Token>::Other(&db).is_data());

        let nop = opcode(Mnemonic::Nop, None, None, None);
        assert!(!AnalysisOp::<Token>::Real(&nop).is_data());
    }
}
