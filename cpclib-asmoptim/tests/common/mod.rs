//! Shared test helper: matching must not depend on which `ListingElement`
//! implementation it runs over.

use cpclib_asmoptim::engine::PeepholeMatch;

/// Everything about a match that carries meaning: which rule fired, where,
/// and - most importantly - the replacement text that would actually be
/// written into the user's file.
type SemanticKey = (Option<String>, usize, usize, usize, Vec<String>);

fn semantic_key(m: &PeepholeMatch) -> SemanticKey {
    (
        m.rule_name.clone(),
        m.start,
        m.end,
        m.anchor,
        m.replacement.clone()
    )
}

/// Assert two runs over the same source - one as `LocatedToken`, one as plain
/// `Token` - agree.
///
/// `PeepholeMatch::message` is compared *loosely* (only its presence), and
/// that exclusion is deliberate and narrow. The message embeds the captured
/// operand's own text, and a numeric literal renders differently depending on
/// the token type: `LocatedToken` keeps the source spelling (`0`), a plain
/// `Expr` renders canonically (`0x0`). That is a `cpclib-tokens` `Display`
/// property, not an engine behavior - and the version the LSP and
/// `cpclib-basmopt` actually show users is the source-preserving one.
///
/// What must *not* differ, and is compared strictly here, is the
/// `replacement`: that text gets written into real files, and a symbolic
/// operand surviving it verbatim is the exact bug class this whole engine has
/// been burned by before (see `engine_matching.rs`'s
/// `a_symbolic_operand_keeps_its_original_spelling_in_the_replacement`).
pub fn assert_token_kinds_agree(located: &[PeepholeMatch], simple: &[PeepholeMatch], source: &str) {
    let a: Vec<SemanticKey> = located.iter().map(semantic_key).collect();
    let b: Vec<SemanticKey> = simple.iter().map(semantic_key).collect();
    assert_eq!(
        a, b,
        "LocatedToken and Token matching must agree for {source:?}"
    );
    for (l, s) in located.iter().zip(simple) {
        assert!(
            !l.message.is_empty() && !s.message.is_empty(),
            "both token kinds must produce a message for {source:?}"
        );
    }
}
