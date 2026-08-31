//! The whole crate through its public API, the way a consumer will use it:
//! parse source, ask for the built-in rules, get suggestions.
//!
//! Everything else tests a layer in isolation; this is the one that fails if
//! the layers are individually fine but do not compose.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};
use cpclib_asmoptim::{OptimizationGoal, builtin_rules};
use cpclib_tokens::{ToSimpleToken, Token};

mod common;

/// Matches against the real built-in rules twice - once as `LocatedToken`,
/// once as plain `Token` (same AST, no source position at all) - and asserts
/// they agree. See `engine_matching.rs`'s `matches_for` for why this matters:
/// the engine's whole contract is that it only depends on `ListingElement`.
fn suggestions(source: &str, goal: OptimizationGoal) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("source must parse");
    let rules = builtin_rules(goal);

    let located_tokens: Vec<_> = listing.iter().collect();
    let located_result = find_matches(&located_tokens, rules);

    let simple_tokens: Vec<Token> = listing
        .iter()
        .map(|t| t.as_simple_token().into_owned())
        .collect();
    let simple_refs: Vec<&Token> = simple_tokens.iter().collect();
    let simple_result = find_matches(&simple_refs, rules);

    common::assert_token_kinds_agree(&located_result, &simple_result, source);

    located_result
}

#[test]
fn the_default_goal_finds_a_real_optimisation_in_ordinary_source() {
    let found = suggestions(
        "start:\n    ld b, b\n    inc hl\n    ret\n",
        OptimizationGoal::default()
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].rule_name.as_deref(),
        Some("unnecessary-ld-to-itself")
    );
    assert!(
        found[0].replacement.is_empty(),
        "this rule deletes the instruction"
    );
}

#[test]
fn already_optimal_source_produces_no_suggestions() {
    let found = suggestions(
        "start:\n    xor a\n    inc hl\n    ld (hl), a\n    ret\n",
        OptimizationGoal::default()
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_push_pop_pair_is_rewritten_into_two_loads() {
    // Exercises the full chain end to end: multi-line match, two `in(...)`
    // constraints, and `regpair` binding the half-registers the replacement
    // then names.
    let found = suggestions(
        "start:\n    push hl\n    pop de\n    ret\n",
        OptimizationGoal::default()
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].rule_name.as_deref(), Some("regpair-transfer"));
    assert_eq!(found[0].replacement, vec![
        "ld d, h".to_string(),
        "ld e, l".to_string()
    ]);
}

/// The size goal exists to add rules like `jp2jr`; without an assembled
/// context its `reachableByJr` constraint cannot be decided, so it must
/// report nothing rather than guess. Guarding this now means the address
/// support added later has a clear before/after.
#[test]
fn jp_to_jr_stays_silent_until_real_addresses_are_available() {
    let found = suggestions(
        "start:\n    jp target\ntarget:\n    ret\n",
        OptimizationGoal::Size
    );
    assert!(
        !found.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
        "jp2jr must not fire without address information: {found:?}"
    );
}

/// Every suggestion the built-in rules make, on every goal, must itself be
/// valid assembly - the property most likely to break when new rules become
/// executable.
#[test]
fn every_builtin_suggestion_is_valid_assembly() {
    let source = "\
start:
    ld a, 0
    cp 0
    ld b, b
    push hl
    pop de
    and c
    and c
    ld a, c
    neg
    rlc a
    rlc a
    ret
";
    for goal in [
        OptimizationGoal::Neutral,
        OptimizationGoal::Size,
        OptimizationGoal::Speed
    ] {
        for m in suggestions(source, goal) {
            for line in &m.replacement {
                assert!(
                    parse_z80_str(format!(" {line}\n")).is_ok(),
                    "{goal:?}: rule {:?} suggested unparsable {line:?}",
                    m.rule_name
                );
            }
        }
    }
}
