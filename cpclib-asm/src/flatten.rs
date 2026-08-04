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
    use crate::parser::parse_z80_str;

    use super::*;

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
