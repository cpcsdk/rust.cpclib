//! `; noopt` / `; noopt:begin` ... `; noopt:end`: instructions a human has
//! marked as intentionally as-is, exempt from any peephole rewrite - same
//! hard-veto shape as [`crate::smc::protected_tokens`], for a different
//! reason (a human said so, rather than something else pointing into the
//! bytes).
//!
//! Comment text survives all the way into the flattened token stream (see
//! `cpclib-asm`'s `parse_comment`/`parse_line`), so this needs no span or
//! raw-source access - just the same token-index adjacency idiom `smc.rs`
//! already uses for `equ $-1`.

use std::collections::HashSet;

use cpclib_tokens::{DataAccessElem, ListingElement};

enum Marker {
    Single,
    Begin,
    End
}

/// Classify a comment's own text (leading `;` included, as
/// `ListingElement::comment()` returns it) as a pragma, if it is one.
///
/// Exact match only, after lower-casing and trimming the leading `;` and
/// surrounding whitespace (tolerant of whitespace around the internal `:`
/// too) - a comment that merely *mentions* "noopt" in prose must never be
/// mistaken for the pragma.
fn marker_of(comment_text: &str) -> Option<Marker> {
    let trimmed = comment_text.trim_start_matches(';').trim().to_ascii_lowercase();
    if trimmed == "noopt" {
        return Some(Marker::Single);
    }
    let (head, tail) = trimmed.split_once(':')?;
    match (head.trim(), tail.trim()) {
        ("noopt", "begin") => Some(Marker::Begin),
        ("noopt", "end") => Some(Marker::End),
        _ => None
    }
}

/// Token indices a human has explicitly exempted from any rewrite.
///
/// Computed once per matching run, same as [`crate::smc::protected_tokens`].
pub fn protected_tokens<T>(tokens: &[&T]) -> HashSet<usize>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let mut protected = HashSet::new();
    let mut block_start: Option<usize> = None;

    for (index, token) in tokens.iter().enumerate() {
        if !token.is_comment() {
            continue;
        }
        match marker_of(token.comment()) {
            Some(Marker::Single) => {
                // The nearest preceding real instruction - covers both a
                // trailing same-line comment and one on its own line right
                // after the instruction it marks.
                if let Some(target) = tokens[..index].iter().rposition(|t| t.mnemonic().is_some())
                {
                    protected.insert(target);
                }
            },
            Some(Marker::Begin) => {
                // A begin while one is already open is redundant - the
                // outer block already covers everything after it, and
                // over-protecting is the safe direction.
                block_start.get_or_insert(index + 1);
            },
            Some(Marker::End) => {
                if let Some(start) = block_start.take() {
                    protected.extend(start..index);
                }
            },
            None => {}
        }
    }
    // An unclosed `noopt:begin` protects through the end of the file - fail
    // toward protecting too much rather than too little, matching this
    // codebase's `Entry::Unknown`/`Verdict::Unknown` philosophy.
    if let Some(start) = block_start {
        protected.extend(start..tokens.len());
    }
    protected
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
    fn a_trailing_single_line_noopt_comment_protects_the_instruction_it_follows() {
        let protected = protected_of("    xor a ; noopt\n    ret\n");
        assert!(protected.contains(&0), "{protected:?}");
    }

    #[test]
    fn a_standalone_noopt_comment_on_its_own_line_protects_the_preceding_instruction() {
        let protected = protected_of("    xor a\n    ; noopt\n    ret\n");
        assert!(protected.contains(&0), "{protected:?}");
    }

    #[test]
    fn noopt_with_no_preceding_instruction_protects_nothing() {
        let protected = protected_of("; noopt\n    xor a\n");
        assert!(protected.is_empty(), "{protected:?}");
    }

    #[test]
    fn a_noopt_block_protects_every_token_strictly_between_begin_and_end() {
        let protected =
            protected_of("; noopt:begin\n    ld a, 0\n    ld b, 1\n; noopt:end\n    ret\n");
        // Token 0 is the begin marker's own comment, 1/2 the two ld's, 3 the
        // end marker's comment, 4 the ret.
        assert!(protected.contains(&1), "{protected:?}");
        assert!(protected.contains(&2), "{protected:?}");
        assert!(!protected.contains(&0), "the marker itself is not protected: {protected:?}");
        assert!(!protected.contains(&3), "the marker itself is not protected: {protected:?}");
        assert!(!protected.contains(&4), "code after the block is not protected: {protected:?}");
    }

    #[test]
    fn an_unclosed_noopt_block_protects_through_end_of_file() {
        let protected = protected_of("; noopt:begin\n    ld a, 0\n    ld b, 1\n");
        assert!(protected.contains(&1), "{protected:?}");
        assert!(protected.contains(&2), "{protected:?}");
    }

    #[test]
    fn noopt_markers_are_case_and_whitespace_insensitive() {
        assert!(protected_of("    xor a ; NoOpt\n    ret\n").contains(&0));
        let block = protected_of("; noopt : BEGIN\n    ld a, 0\n; NOOPT: end\n    ret\n");
        assert!(block.contains(&1), "{block:?}");
    }

    #[test]
    fn a_comment_merely_mentioning_noopt_in_prose_is_not_treated_as_a_pragma() {
        let protected =
            protected_of("    xor a ; this used to be noopt but isn't anymore\n    ret\n");
        assert!(protected.is_empty(), "{protected:?}");
    }

    #[test]
    fn noopt_and_smc_protection_can_coexist_on_the_same_index() {
        // `.activated equ $-1` is already smc-protected; adding a `; noopt`
        // on the same instruction must not panic or misbehave - the union
        // is just a `HashSet::extend`.
        let listing = parse_z80_str("    ld a, 0 ; noopt\n    .activated equ $-1\n    ret\n")
            .expect("source must parse");
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let mut protected = crate::smc::protected_tokens(&tokens);
        protected.extend(protected_tokens(&tokens));
        assert!(protected.contains(&0), "{protected:?}");
    }
}
