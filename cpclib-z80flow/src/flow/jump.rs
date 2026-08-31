//! Reading a jump-like instruction's operands: which slot holds the target,
//! and whether a flag condition guards it.
//!
//! Small, but the knowledge in here is exactly the kind that drifts when it is
//! written twice - and it *was* written twice, once for the block-level CFG and
//! once for the optimizer's op-level walk, each having independently
//! rediscovered the same parser quirk about which operand slot a target lands
//! in. One implementation now, so a parser change breaks one place instead of
//! silently disagreeing in two.
//!
//! Written against operand *slots* rather than against any container, because
//! the two callers hold their operands differently: `ListingElement` hands out
//! `Option<&T::DataAccess>`, while `AnalysisOp` normalizes to
//! `Option<Cow<'_, DataAccess>>`. Both reach this through
//! `Option<&impl DataAccessElem>`.

use cpclib_tokens::{DataAccessElem, ExprElement};

/// Which operand is the jump target, and whether reaching it is conditional.
///
/// The quirk worth stating once: for a conditional jump (`JR cc, label`) the
/// flag test is `arg1` and the target is `arg2`, but for an unconditional one
/// (`JR label`) the parser *still* puts the sole operand in `arg2` and leaves
/// `arg1` empty. Both shapes are accepted here, along with the
/// operand-in-`arg1` form, so a future parser change in either direction does
/// not silently produce "this jump has no target" - which, in every consumer,
/// degrades to "control flow is unknown here" rather than to a visible error.
///
/// Verified against real parser output, not assumed: see
/// `cpclib-z80flow-tests`, which may depend on a parser where this crate
/// deliberately may not.
pub(crate) fn condition_and_target<'d, D: DataAccessElem>(
    arg1: Option<&'d D>,
    arg2: Option<&'d D>
) -> Option<(bool, &'d D)> {
    match (arg1, arg2) {
        (Some(a1), Some(a2)) if a1.is_flag_test() => Some((true, a2)),
        (None, Some(a2)) => Some((false, a2)),
        (Some(a1), None) => Some((false, a1)),
        _ => None
    }
}

/// The label an operand names, when it names one directly.
///
/// `None` for anything computed - `jp (hl)`, an expression, an indirect target.
/// Every consumer treats that as "the flow leaves this analysis's view", which
/// is the safe reading.
pub(crate) fn label_of<D: DataAccessElem>(operand: &D) -> Option<&str> {
    operand
        .get_expression()
        .filter(|e| e.is_label())
        .map(|e| e.label())
}

/// Is this instruction guarded by a flag test?
///
/// Separate from [`condition_and_target`] because `RET cc` is conditional while
/// having no target at all - its flag test is the *only* operand, which the
/// target decoder correctly reads as "one operand, no condition". Asking the
/// target decoder about a `RET`'s conditionality therefore answers `false`, and
/// did, until two `branch_balance` tests said otherwise.
pub(crate) fn is_conditional<D: DataAccessElem>(arg1: Option<&D>) -> bool {
    arg1.is_some_and(|a| a.is_flag_test())
}

/// `DJNZ`'s sole operand is always its loop target - there is no flag test to
/// tell apart, unlike `JR`/`RET`.
pub(crate) fn djnz_target<D: DataAccessElem>(arg1: Option<&D>) -> Option<&str> {
    label_of(arg1?)
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::{DataAccess, Expr, FlagTest};

    use super::*;

    /// Operands built by hand rather than parsed. This crate sits below the
    /// assembler and may not depend on it, not even for tests - the same
    /// decoding *is* checked against real parser output, over in
    /// `cpclib-z80flow-tests`, where depending on a parser is allowed.
    fn label(name: &str) -> DataAccess {
        DataAccess::Expression(Expr::Label(name.into()))
    }

    fn flag() -> DataAccess {
        DataAccess::FlagTest(FlagTest::NZ)
    }

    /// The three operand shapes a jump can arrive in, and which slot each one
    /// puts the target in.
    #[test]
    fn the_target_slot_differs_between_conditional_and_unconditional_jumps() {
        let (target_arg, flag_arg) = (label("target"), flag());

        // `jr cc, label`: flag test first, target second.
        let (conditional, target) =
            condition_and_target(Some(&flag_arg), Some(&target_arg)).unwrap();
        assert!(conditional);
        assert_eq!(label_of(target), Some("target"));

        // `jr label`: the parser leaves arg1 empty and puts the sole operand
        // in arg2 - the quirk this function exists to absorb.
        let (conditional, target) = condition_and_target(None, Some(&target_arg)).unwrap();
        assert!(!conditional);
        assert_eq!(label_of(target), Some("target"));

        // The mirror shape, accepted so a parser change in the other direction
        // does not silently read as "no target".
        let (conditional, target) = condition_and_target(Some(&target_arg), None).unwrap();
        assert!(!conditional);
        assert_eq!(label_of(target), Some("target"));
    }

    /// A computed target names no label, and saying so is what makes every
    /// consumer fall back to "flow unknown" instead of inventing an edge.
    #[test]
    fn a_computed_target_names_no_label() {
        let computed = DataAccess::MemoryRegister16(cpclib_tokens::Register16::Hl);
        assert_eq!(label_of(&computed), None);

        let arithmetic = DataAccess::Expression(Expr::Value(0x4000));
        assert_eq!(label_of(&arithmetic), None);
    }

    /// `RET cc` carries a flag test and nothing else - the case that made
    /// [`is_conditional`] a separate question from [`condition_and_target`].
    #[test]
    fn a_lone_flag_test_is_a_condition_but_not_a_target() {
        let flag_arg = flag();
        assert!(is_conditional(Some(&flag_arg)));
        assert_eq!(label_of(&flag_arg), None);

        // ...and the target decoder reads it as a one-operand unconditional
        // form, which is exactly why asking it about `RET` gave the wrong
        // answer.
        let (conditional, _) = condition_and_target(Some(&flag_arg), None).unwrap();
        assert!(!conditional);
    }

    /// No operands at all: a plain `RET`.
    #[test]
    fn no_operands_is_neither_conditional_nor_a_target() {
        assert!(!is_conditional(None::<&DataAccess>));
        assert!(condition_and_target(None::<&DataAccess>, None).is_none());
    }

    #[test]
    fn djnz_reads_its_target_from_the_only_slot_it_has() {
        let target_arg = label("loop");
        assert_eq!(djnz_target(Some(&target_arg)), Some("loop"));
        assert_eq!(djnz_target(None::<&DataAccess>), None);
    }
}
