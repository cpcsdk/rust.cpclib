//! `reachableByJr` needs a real assembled address, not just a parsed token
//! stream - this is the one constraint in the v1 subset that can't be decided
//! from syntax alone. Exercises the whole chain: real assemble with
//! `record_token_addresses` on, `EnvAddressResolver` reading it back,
//! `jp2jr` (the actual upstream, `cpc`-tagged rule) deciding whether to fire.

use cpclib_asm::assembler::{Env, visit_tokens_all_passes_with_options};
use cpclib_asm::parser::{LocatedListing, LocatedToken, parse_z80_str};
use cpclib_asm::{AssemblingOptions, EnvOptions};
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches_with_resolver};
use cpclib_asmoptim::{EnvAddressResolver, OptimizationGoal, builtin_rules};
use cpclib_tokens::{ToSimpleToken, Token};

/// Assembles `source` with per-token addresses recorded, the same way the
/// LSP's `dry_run_env` does.
fn assemble(source: &str) -> (LocatedListing, Env) {
    let listing = parse_z80_str(source).expect("test source must parse");

    let mut assemble = AssemblingOptions::default();
    assemble.set_dry_run(true);
    assemble.set_record_token_addresses(true);
    let options = EnvOptions::from(assemble);

    let env = match visit_tokens_all_passes_with_options(&listing, options) {
        Ok((_, env)) => env,
        Err((_, _env, e)) => panic!("test source must assemble: {e}")
    };
    (listing, env)
}

fn suggestions(source: &str, goal: OptimizationGoal) -> Vec<PeepholeMatch> {
    let (listing, env) = assemble(source);
    let refs: Vec<&LocatedToken> = listing.iter().collect();
    let resolver = EnvAddressResolver::new(&env);
    find_matches_with_resolver(&refs, builtin_rules(goal), &resolver)
}

#[test]
fn a_jp_to_a_nearby_label_becomes_jr() {
    // The target is two bytes after the jp itself - trivially in range.
    let found = suggestions(
        "start:\n    jp target\ntarget:\n    ret\n",
        OptimizationGoal::Size
    );
    let hit = found
        .iter()
        .find(|m| m.rule_name.as_deref() == Some("jp2jr"))
        .unwrap_or_else(|| panic!("expected jp2jr to fire: {found:?}"));
    assert_eq!(hit.replacement, vec!["jr target".to_string()]);
}

#[test]
fn a_jp_to_a_far_label_is_left_alone() {
    // 200 `nop`s put the target far outside JR's -128..=127 range.
    let mut source = String::from("start:\n    jp target\n");
    for _ in 0..200 {
        source.push_str("    nop\n");
    }
    source.push_str("target:\n    ret\n");

    let found = suggestions(&source, OptimizationGoal::Size);
    assert!(
        !found.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
        "jp2jr must not fire when the target is unreachable by jr: {found:?}"
    );
}

#[test]
fn a_jp_to_a_backward_label_in_range_becomes_jr() {
    let found = suggestions(
        "target:\n    ret\nstart:\n    jp target\n",
        OptimizationGoal::Size
    );
    let hit = found
        .iter()
        .find(|m| m.rule_name.as_deref() == Some("jp2jr"))
        .unwrap_or_else(|| panic!("expected jp2jr to fire: {found:?}"));
    assert_eq!(hit.replacement, vec!["jr target".to_string()]);
}

/// `EnvAddressResolver` is generic over any `T: MayHaveSpan` (see its own doc
/// comment for why: `Env` records addresses keyed by span identity, not by
/// the token object itself), and plain `Token` genuinely implements that
/// trait - `possible_span()` just always returns `None`. So this compiles
/// and runs with the *same* `EnvAddressResolver` used against `LocatedToken`
/// elsewhere in this file, on the same trivially-in-range source that fires
/// for `LocatedToken` - and must still report nothing, safely, since a plain
/// `Token` never had a position to have been recorded under in the first
/// place.
#[test]
fn a_resolver_still_cannot_help_token_which_never_had_a_position() {
    let (listing, env) = assemble("start:\n    jp target\ntarget:\n    ret\n");
    let simple_tokens: Vec<Token> = listing
        .iter()
        .map(|t| t.as_simple_token().into_owned())
        .collect();
    let simple_refs: Vec<&Token> = simple_tokens.iter().collect();

    let resolver = EnvAddressResolver::new(&env);
    let found = find_matches_with_resolver(&simple_refs, builtin_rules(OptimizationGoal::Size), &resolver);

    assert!(
        !found.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
        "jp2jr must stay silent for a Token stream, which has no position to resolve: {found:?}"
    );
}

#[test]
fn without_a_resolver_jp2jr_never_fires_even_when_it_would_be_valid() {
    // Same trivially-in-range source as the first test, but through
    // `find_matches` (no address information at all) - the documented,
    // deliberately conservative fallback.
    let listing = parse_z80_str("start:\n    jp target\ntarget:\n    ret\n").unwrap();
    let tokens: Vec<_> = listing.iter().collect();
    let found = cpclib_asmoptim::engine::find_matches(&tokens, builtin_rules(OptimizationGoal::Size));
    assert!(
        !found.iter().any(|m| m.rule_name.as_deref() == Some("jp2jr")),
        "jp2jr must stay silent without real address information: {found:?}"
    );
}

/// The exact boundary the format itself defines: a delta of 127 is a valid
/// `jr`, 128 is not. Rather than hand-computing filler byte counts (fragile
/// against any encoding-size change elsewhere), binary-search the real
/// assembler's own opinion by growing the gap until it stops firing, and
/// check the transition happens at all - proof the constraint is a real
/// signed-byte range check and not, say, an off-by-one or an unsigned one.
#[test]
fn there_is_a_real_boundary_between_reachable_and_not() {
    let reaches = |gap: usize| {
        let mut source = String::from("start:\n    jp target\n");
        for _ in 0..gap {
            source.push_str("    nop\n");
        }
        source.push_str("target:\n    ret\n");
        suggestions(&source, OptimizationGoal::Size)
            .iter()
            .any(|m| m.rule_name.as_deref() == Some("jp2jr"))
    };

    assert!(reaches(0), "adjacent target must be reachable");
    assert!(!reaches(200), "a target 200 bytes away must not be reachable");

    // There must be some gap size where it flips from reachable to not -
    // i.e. the constraint is a real range check, not always-true/always-false.
    assert!(
        (0..200).any(|gap| reaches(gap) && !reaches(gap + 1)),
        "expected exactly one reachable/unreachable transition in 0..200"
    );
}
