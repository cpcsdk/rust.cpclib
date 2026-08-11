//! Instructions that something else points *into*, and so must not be
//! rewritten.
//!
//! Self-modifying code is ordinary practice in CPC demos, not an exotic case:
//! a routine patches the immediate operand of an instruction elsewhere rather
//! than paying for a memory read. Two idioms cover almost all of it, and both
//! appear in real sources in this workspace:
//!
//! ```text
//! phase_  ld a, 0                    ; ... patched via `ld (phase_+1), a`
//!         ld a, 0 : .activated equ $-1   ; the equ names the operand byte
//! ```
//!
//! The second form exists precisely so callers do not hard-code the `-1`,
//! which would rot as the instruction changes - so it is, if anything, a sign
//! the author expects the surrounding code to be edited.
//!
//! Either way the *byte layout* is load-bearing. Every optimization this crate
//! suggests changes it: `ld a,0` -> `xor a` is 2 bytes to 1, so `$-1` and
//! `phase_+1` end up pointing at whatever follows instead. Nothing about that
//! failure is visible at assembly time - the file still assembles, and the
//! demo misbehaves at run time.
//!
//! So a rewrite is declined outright for any instruction something points
//! into. This is deliberately a veto rather than a constraint on individual
//! rules: it has nothing to do with what a rule does, and applies equally to
//! every one of them.

use std::collections::{HashMap, HashSet};

use cpclib_tokens::{DataAccessElem, ExprElement, ListingElement};

/// Token indices that must never be part of a suggested rewrite.
///
/// Computed once per matching run - it is a whole-file property, and a match
/// cannot decide it locally.
pub fn protected_tokens<T>(tokens: &[&T]) -> HashSet<usize>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let mut protected = HashSet::new();

    // Which instruction each label marks. A label is its own token, so the
    // instruction it names is the next one that actually is an instruction.
    let mut labelled: HashMap<&str, usize> = HashMap::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.is_label() {
            let name = token.label_symbol();
            if let Some(target) = (index + 1..tokens.len()).find(|i| tokens[*i].mnemonic().is_some())
            {
                labelled.insert(name, target);
            }
        }
    }

    for (index, token) in tokens.iter().enumerate() {
        // `equ $-1` and friends: `$` is the address *after* the instruction
        // just emitted, so any backwards offset from it lands inside that
        // instruction.
        //
        // Only the immediately preceding instruction is protected. An offset
        // large enough to reach past it (`$-4` over two instructions) is not
        // covered - that would need real instruction sizes, which are not
        // available here. The idiom in practice names an operand of the
        // instruction it sits next to.
        // `equ` and `=` carry their value on different accessors, and each
        // panics on the other's token kind - so the kind has to be resolved
        // before reading the expression, not after.
        let defined_value = if token.is_equ() {
            Some(token.equ_value())
        }
        else if token.is_assign() {
            Some(token.assign_value())
        }
        else {
            None
        };

        if let Some(value) = defined_value
            && mentions_current_address(value)
            && let Some(previous) = tokens[..index]
                .iter()
                .rposition(|t| t.mnemonic().is_some())
        {
            protected.insert(previous);
        }

        // `ld (phase_+1), a`: a label used in *arithmetic* is being treated as
        // an address to compute from, not as a jump target. A bare `jp label`
        // is a plain label expression and does not trigger this.
        for arg in [token.mnemonic_arg1(), token.mnemonic_arg2()]
            .into_iter()
            .flatten()
        {
            if let Some(expr) = arg.get_expression() {
                let mut names = Vec::new();
                labels_in_arithmetic(expr, false, &mut names);
                for name in names {
                    if let Some(target) = labelled.get(name.as_str()) {
                        protected.insert(*target);
                    }
                }
            }
        }
    }

    protected
}

/// Does this expression mention `$`, the current output address?
fn mentions_current_address<E: ExprElement>(expr: &E) -> bool {
    if expr.is_label() {
        return expr.label() == "$";
    }
    if expr.is_binary_operation() {
        return mentions_current_address(expr.arg1()) || mentions_current_address(expr.arg2());
    }
    if expr.is_unary_operation() {
        return mentions_current_address(expr.arg1());
    }
    false
}

/// Collect every label appearing *inside an arithmetic expression*, which is
/// what distinguishes "address of this instruction plus an offset" from a
/// plain symbolic reference.
fn labels_in_arithmetic<E: ExprElement>(expr: &E, inside_arithmetic: bool, out: &mut Vec<String>) {
    if expr.is_label() {
        if inside_arithmetic {
            out.push(expr.label().to_string());
        }
        return;
    }
    if expr.is_binary_operation() {
        labels_in_arithmetic(expr.arg1(), true, out);
        labels_in_arithmetic(expr.arg2(), true, out);
        return;
    }
    if expr.is_unary_operation() {
        labels_in_arithmetic(expr.arg1(), inside_arithmetic, out);
    }
}

#[cfg(test)]
mod tests {
    use cpclib_asm::flatten::flatten_for_analysis;
    use cpclib_asm::parser::{LocatedToken, parse_z80_str};

    use super::*;

    fn protected_of(source: &str) -> HashSet<usize> {
        let listing = parse_z80_str(source).expect("source must parse");
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        protected_tokens(&tokens)
    }

    #[test]
    fn an_equ_naming_the_previous_operand_protects_that_instruction() {
        // The user's own idiom: the `equ` exists so callers never hard-code
        // the offset.
        let protected = protected_of("    ld a, 0 : .activated equ $-1\n    ret\n");
        assert!(
            protected.contains(&0),
            "the ld must be protected: {protected:?}"
        );
    }

    #[test]
    fn a_label_used_with_an_offset_protects_the_instruction_it_marks() {
        let protected = protected_of("phase_\n    ld a, 0\n    ld (phase_+1), a\n    ret\n");
        // Token 0 is the label, token 1 the instruction it marks.
        assert!(
            protected.contains(&1),
            "the labelled ld must be protected: {protected:?}"
        );
    }

    #[test]
    fn an_assignment_naming_the_previous_operand_protects_it_too() {
        // `=` is a different token kind from `equ` and reads its value through
        // a different accessor - each panics if handed the other's kind, so
        // this is not merely a second spelling of the test above.
        let protected = protected_of("    ld a, 0\nactivated = $-1\n    ret\n");
        assert!(
            protected.contains(&0),
            "the ld must be protected: {protected:?}"
        );
    }

    /// A file mixing `equ`, `=`, labels and ordinary instructions must simply
    /// not panic - the accessors involved are variant-specific and several of
    /// them are `unreachable!()` for the wrong kind.
    #[test]
    fn a_listing_mixing_every_definition_kind_is_scanned_without_panicking() {
        let protected = protected_of(
            "count = 5\nother equ 3\nstart\n    ld a, 0\n    ld b, count\n    jp start\n"
        );
        assert!(protected.is_empty(), "{protected:?}");
    }

    #[test]
    fn ordinary_code_is_not_protected() {
        // Nothing points into any of these, so none may be held back.
        assert!(protected_of("    ld a, 0\n    ld b, 1\n    ret\n").is_empty());
    }

    #[test]
    fn a_plain_symbolic_jump_is_not_arithmetic_and_does_not_protect() {
        // `jp target` names an address; it does not point *into* an
        // instruction's encoding, so rewriting around it stays allowed - this
        // is what keeps `jp2jr` working on ordinary code.
        let protected = protected_of("target\n    ld a, 0\n    jp target\n");
        assert!(protected.is_empty(), "{protected:?}");
    }
}
