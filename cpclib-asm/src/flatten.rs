//! Flattening a parsed listing so a whole-document pass sees every
//! instruction, not just the top-level ones.
//!
//! `listing.iter()` alone only yields the *top-level* statements - a file
//! entirely wrapped in an `ifndef GUARD ... endif` header guard (extremely
//! common in real-world sources) would otherwise expose exactly one top-level
//! `IF` token and none of the instructions inside it.
//!
//! Lives here (not in a consumer like `cpclib-lsp` or `cpclib-asmoptim`) so
//! every consumer shares one implementation instead of each maintaining its
//! own copy - it only depends on `ListingElement`, nothing assembler- or
//! consumer-specific.

use cpclib_tokens::ListingElement;

/// Recursively flatten a parsed listing, descending into every kind of
/// nested block (`IF`/`IFDEF`/`IFNDEF`, `MODULE`, `REPEAT`/`REPEAT...UNTIL`,
/// `WHILE`, `RORG`, `FOR`, `ITERATE`, `SWITCH`, `CONFINED`, crunched
/// sections, assembler-control blocks).
///
/// Deliberately does **not** descend into `INCLUDE` - included files are not
/// part of this listing's own token tree at parse time (they only become
/// visible during a real assemble, as a separate nested listing owned by the
/// assembler's include-handling state), so there is nothing here to flatten
/// into in the first place.
///
/// Lazy - every caller so far only needs a single pass, so this never pays
/// for an eager `Vec` allocation. Assumes each token belongs to at most one
/// of the nested-block categories below (true in practice for real parser
/// output - a token can't simultaneously be a `MODULE` and an `IF`, say);
/// harmless unless that assumption is ever violated, in which case this
/// would visit fewer nested tokens than checking each independently would.
pub fn flatten_listing<'a, T>(
    tokens: impl IntoIterator<Item = &'a T> + 'a
) -> Box<dyn Iterator<Item = &'a T> + 'a>
where T: ListingElement + 'a {
    Box::new(tokens.into_iter().flat_map(flatten_one))
}

/// Flatten a listing for **dataflow analysis**, descending only into blocks
/// whose contents really do execute in the order they are flattened into.
///
/// [`flatten_listing`] is a *tree traversal*: it visits every token, which is
/// what a symbol collector or a "count the cycles in this selection" pass
/// wants. It is **not** an execution order, and using it as one is unsound.
/// An `IF` is the clearest case - it chains each test branch and then the else
/// branch into one sequence, but at most one of those ever runs. A liveness
/// walk over that sees the `else` arm's instructions apparently executing
/// immediately after the `if` arm's, and concludes registers written in one
/// arm kill values live in the other. In real CPC sources that is not a corner
/// case: the idiom
///
/// ```text
///     IFNDEF PLY_AKG_Rom
/// Counter: ld a,0          ; patched at run time
///     ELSE
///     ld a,(Counter)
///     ENDIF
///     sub 1                ; ... and A is read right here
/// ```
///
/// made a peephole rule report the `ld a,0` as dead code, in a music player
/// where deleting it breaks playback.
///
/// So this descends only into blocks that are pure *containers* - `MODULE`,
/// `RORG`, `CONFINED`, crunched sections, assembler-control blocks - whose
/// bodies execute exactly once, in order, right where they appear.
///
/// Everything else is left opaque, yielding just the block token itself:
///
/// * `IF`/`SWITCH` - branches are mutually exclusive, as above.
/// * `REPEAT`/`REPEAT...UNTIL`/`WHILE`/`FOR`/`ITERATE` - the body is flattened
///   once, with no back edge, so a walk starting inside the body would never
///   see the reads that happen on the next iteration.
/// * function definitions - the body is evaluated by the assembler, not
///   executed in place at all.
///
/// An opaque block token is not a *gap*: it is still yielded, so an analysis
/// that reaches one sees an instruction it cannot interpret and can fail
/// closed, rather than silently stepping over the block's contents.
///
/// The cost is coverage, not correctness - no optimization is suggested for
/// code inside a conditional. Regaining that means modelling each branch as
/// its own region and stopping a walk at the boundary, which is a strictly
/// larger change than making the unsound cases opaque.
pub fn flatten_for_analysis<'a, T>(
    tokens: impl IntoIterator<Item = &'a T> + 'a
) -> Box<dyn Iterator<Item = &'a T> + 'a>
where T: ListingElement + 'a {
    Box::new(tokens.into_iter().flat_map(flatten_one_sequential))
}

fn flatten_one_sequential<'a, T>(token: &'a T) -> Box<dyn Iterator<Item = &'a T> + 'a>
where T: ListingElement + 'a {
    let nested: Box<dyn Iterator<Item = &'a T> + 'a> = if token.is_module() {
        flatten_for_analysis(token.module_listing())
    }
    else if token.is_rorg() {
        flatten_for_analysis(token.rorg_listing())
    }
    else if token.is_confined() {
        flatten_for_analysis(token.confined_listing())
    }
    else if token.is_crunched_section() {
        flatten_for_analysis(token.crunched_section_listing())
    }
    else if token.is_assembler_control() {
        flatten_for_analysis(token.assembler_control_get_listing())
    }
    else {
        Box::new(std::iter::empty())
    };
    Box::new(std::iter::once(token).chain(nested))
}

fn flatten_one<'a, T>(token: &'a T) -> Box<dyn Iterator<Item = &'a T> + 'a>
where T: ListingElement + 'a {
    let nested: Box<dyn Iterator<Item = &'a T> + 'a> = if token.is_module() {
        flatten_listing(token.module_listing())
    }
    else if token.is_if() {
        let tests: Box<dyn Iterator<Item = &'a T> + 'a> = Box::new(
            (0..token.if_nb_tests()).flat_map(move |i| flatten_listing(token.if_test(i).1))
        );
        let else_branch: Box<dyn Iterator<Item = &'a T> + 'a> = match token.if_else() {
            Some(l) => flatten_listing(l),
            None => Box::new(std::iter::empty())
        };
        Box::new(tests.chain(else_branch))
    }
    else if token.is_repeat() {
        flatten_listing(token.repeat_listing())
    }
    else if token.is_repeat_until() {
        flatten_listing(token.repeat_until_listing())
    }
    else if token.is_while() {
        flatten_listing(token.while_listing())
    }
    else if token.is_rorg() {
        flatten_listing(token.rorg_listing())
    }
    else if token.is_for() {
        flatten_listing(token.for_listing())
    }
    else if token.is_function_definition() {
        flatten_listing(token.function_definition_inner())
    }
    else if token.is_iterate() {
        flatten_listing(token.iterate_listing())
    }
    else if token.is_switch() {
        let cases: Vec<_> = token.switch_cases().collect();
        let cases_iter: Box<dyn Iterator<Item = &'a T> + 'a> =
            Box::new(cases.into_iter().flat_map(|(_, l, _)| flatten_listing(l)));
        let default_iter: Box<dyn Iterator<Item = &'a T> + 'a> = match token.switch_default() {
            Some(l) => flatten_listing(l),
            None => Box::new(std::iter::empty())
        };
        Box::new(cases_iter.chain(default_iter))
    }
    else if token.is_confined() {
        flatten_listing(token.confined_listing())
    }
    else if token.is_crunched_section() {
        flatten_listing(token.crunched_section_listing())
    }
    else if token.is_assembler_control() {
        flatten_listing(token.assembler_control_get_listing())
    }
    else {
        Box::new(std::iter::empty())
    };
    Box::new(std::iter::once(token).chain(nested))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_z80_str;

    #[test]
    fn flattening_descends_into_an_if_block() {
        let listing = parse_z80_str("ifndef GUARD\n cp 0\nendif\n").unwrap();
        let top_level_count = listing.iter().count();
        let flat_count = flatten_listing(listing.iter()).count();
        assert!(
            flat_count > top_level_count,
            "flattening must see the cp 0 hidden inside the if block"
        );
    }

    #[test]
    fn flattening_a_listing_with_no_nested_blocks_matches_the_top_level_count() {
        let listing = parse_z80_str(" nop\n nop\n ret\n").unwrap();
        assert_eq!(
            listing.iter().count(),
            flatten_listing(listing.iter()).count()
        );
    }
}

#[cfg(test)]
mod analysis_flattening_tests {
    use cpclib_tokens::ListingElement;

    use super::{flatten_for_analysis, flatten_listing};
    use crate::parser::{LocatedToken, parse_z80_str};

    /// The exact real-world shape that motivated `flatten_for_analysis`: two
    /// mutually exclusive arms writing the same register, followed by a read.
    const CONDITIONAL: &str = "\
    IFNDEF ROM
counter: ld a, 0
    ELSE
    ld a, (counter)
    ENDIF
    sub 1
    ret
";

    #[test]
    fn the_tree_traversal_puts_both_arms_of_a_conditional_in_sequence() {
        // Not a complaint about `flatten_listing` - this is exactly what it
        // promises, and what its other callers need. Asserted here so the
        // difference between the two functions is visible in one place.
        let listing = parse_z80_str(CONDITIONAL).unwrap();
        let flat: Vec<&LocatedToken> = flatten_listing(listing.iter()).collect();
        let opcodes = flat.iter().filter(|t| t.is_opcode()).count();
        assert_eq!(
            opcodes, 4,
            "both arms plus the two trailing instructions: {flat:?}"
        );
    }

    #[test]
    fn the_execution_flattening_leaves_a_conditional_opaque() {
        // Only the instructions that genuinely follow one another are yielded;
        // the `IF` itself is still present, so a consumer meets a token it
        // cannot interpret rather than silently skipping the arms.
        let listing = parse_z80_str(CONDITIONAL).unwrap();
        let flat: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let opcodes = flat.iter().filter(|t| t.is_opcode()).count();
        assert_eq!(opcodes, 2, "only `sub 1` and `ret`: {flat:?}");
        assert!(
            flat.iter().any(|t| t.is_if()),
            "the block token itself must still be yielded: {flat:?}"
        );
    }

    #[test]
    fn a_sequential_container_is_still_descended_into() {
        // A `MODULE` really does execute in place and in order, so leaving it
        // opaque would cost coverage for no safety gain - which is what
        // separates it from the conditional above.
        let listing = parse_z80_str("    MODULE m\n    ld a, 1\n    ret\n    ENDMODULE\n").unwrap();
        let flat: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        assert_eq!(flat.iter().filter(|t| t.is_opcode()).count(), 2, "{flat:?}");
    }

    #[test]
    fn a_loop_body_is_left_opaque() {
        // The body is flattened once with no back edge, so a forward walk
        // starting inside it would never see the next iteration's reads.
        let listing = parse_z80_str("    REPEAT 4\n    ld a, 1\n    REND\n    ret\n").unwrap();
        let flat: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        assert_eq!(
            flat.iter().filter(|t| t.is_opcode()).count(),
            1,
            "only the trailing `ret`: {flat:?}"
        );
    }
}
