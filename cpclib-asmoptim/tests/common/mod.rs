//! Shared test helper: matching must not depend on which `ListingElement`
//! implementation it runs over.

use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::{LocatedToken, parse_z80_str};
use cpclib_asmoptim::engine::PeepholeMatch;
use cpclib_tokens::{ListingElement, Token};

/// Parse a replacement back into the instructions it denotes.
///
/// This is what makes the comparison below meaningful. A suggestion produced
/// from `LocatedToken`s is *supposed* to differ textually from the same
/// suggestion produced from plain `Token`s: the located one is rendered from
/// the user's own source and so keeps their case, their number base (`#7F10`
/// stays `#7F10` rather than becoming `0x7f10`) and their comments, while a
/// spanless `Token` can only be rendered canonically. Comparing the two as
/// *strings* would therefore forbid exactly the fidelity that is wanted.
///
/// What must hold is that both describe the same instructions. Parsing each
/// side and comparing the resulting `Token`s tests that directly, and is
/// immune to spelling.
///
/// A replacement entry may hold several instructions (`rrca : rrca : rrca`),
/// which is why this flattens rather than mapping one-to-one.
fn instructions_of(replacement: &[String]) -> Vec<Token> {
    replacement
        .iter()
        .flat_map(|line| {
            let listing = parse_z80_str(format!("    {line}\n"))
                .unwrap_or_else(|e| panic!("suggested replacement {line:?} does not parse: {e}"));
            let tokens: Vec<Token> = flatten_for_analysis(listing.iter())
                // Comments carry no instruction meaning, and only one side can
                // ever have them - the located rendering preserves the user's,
                // the canonical one has none to preserve. Dropping them is
                // what lets the comparison be about instructions.
                .filter(|t: &&LocatedToken| !t.is_comment())
                .map(|t| t.to_token().into_owned())
                .collect();
            tokens
        })
        .collect()
}

/// Assert two runs over the same source - one as `LocatedToken`, one as plain
/// `Token` - agree on what they found and what they would do about it.
///
/// Everything structural (which rule, where, the anchor) is compared exactly.
/// The replacement is compared *semantically*, via [`instructions_of`]. The
/// message is only checked for presence: it is prose with operand text spliced
/// into it, so it is neither parsable nor required to match spelling for
/// spelling.
pub fn assert_token_kinds_agree(located: &[PeepholeMatch], simple: &[PeepholeMatch], source: &str) {
    let structure = |m: &PeepholeMatch| (m.rule_name.clone(), m.start, m.end, m.anchor);
    let a: Vec<_> = located.iter().map(structure).collect();
    let b: Vec<_> = simple.iter().map(structure).collect();
    assert_eq!(
        a, b,
        "LocatedToken and Token matching must agree for {source:?}"
    );

    for (l, s) in located.iter().zip(simple) {
        assert_eq!(
            instructions_of(&l.replacement),
            instructions_of(&s.replacement),
            "the two token kinds must suggest the same instructions for {source:?}\n\
             located: {:?}\n  plain: {:?}",
            l.replacement,
            s.replacement
        );
        assert!(
            !l.message.is_empty() && !s.message.is_empty(),
            "both token kinds must produce a message for {source:?}"
        );
    }
}
