//! End-to-end matching tests: real assembly source, parsed by the real
//! `cpclib-asm` parser, matched against real upstream optimization patterns.
//!
//! Deliberately not built on synthetic hand-made token structures - the whole
//! risk in a pattern matcher is the gap between what a pattern *looks* like it
//! says and what the assembler's AST actually holds for that source text, so
//! every case here starts from source a user could really write.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};
use cpclib_tokens::{ToSimpleToken, Token};

mod common;

/// Parse `source`, match it against `rules` twice - once as the real
/// `LocatedToken`s the LSP works with, once as plain `Token`s (the same AST
/// with all span/source-position information stripped) - and assert the two
/// runs agree exactly.
///
/// The engine's whole contract is that it only depends on `ListingElement`,
/// never on anything span-specific; running every case through both token
/// types is what actually proves that rather than merely asserting it in a
/// doc comment. `Token` is what the engine has to cope with when a caller
/// hands it synthesized/reconstructed instructions with no source position at
/// all (that's also, not coincidentally, exactly the case a `reachableByJr`-
/// style constraint must degrade to "unknown" for - see
/// `tests/reachable_by_jr.rs`).
fn matches_for(source: &str, rules: &str) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("test source must parse");
    let rules = RuleSet::parse(rules).expect("test rules must parse");

    let located_tokens: Vec<_> = listing.iter().collect();
    let located_result = find_matches(&located_tokens, &rules);

    let simple_tokens: Vec<Token> = listing
        .iter()
        .map(|t| t.as_simple_token().into_owned())
        .collect();
    let simple_refs: Vec<&Token> = simple_tokens.iter().collect();
    let simple_result = find_matches(&simple_refs, &rules);

    common::assert_token_kinds_agree(&located_result, &simple_result, source);

    located_result
}

/// The real upstream `cp02ora` rule, minus its `flagsNotUsedAfter` constraint
/// (not evaluated yet - a rule carrying it is skipped entirely, which is
/// itself asserted further down).
const CP_ZERO: &str = "\
pattern: Replace cp 0 with or a
name: cp02ora
0: cp 0
replacement:
0: or a
";

/// The real upstream `unnecessary-ld-to-itself` rule, verbatim including its
/// `in(...)` constraint - fully supported today.
const LD_SELF: &str = "\
pattern: Remove ld ?reg,?reg
name: unnecessary-ld-to-itself
0: ld ?reg,?reg
replacement:
constraints:
in(?reg,A,B,C,D,E,H,L)
";

#[test]
fn matches_cp_zero_and_suggests_or_a() {
    let found = matches_for(" cp 0\n ret\n", CP_ZERO);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].rule_name.as_deref(), Some("cp02ora"));
    assert_eq!(found[0].replacement, vec!["or a".to_string()]);
    assert_eq!(found[0].range(), 0..1);
    assert_eq!(found[0].anchor, 0);
}

#[test]
fn does_not_match_cp_with_a_different_operand() {
    assert!(matches_for(" cp 1\n", CP_ZERO).is_empty());
    assert!(matches_for(" cp b\n", CP_ZERO).is_empty());
}

#[test]
fn a_repeated_capture_requires_the_same_register_in_both_slots() {
    // `ld ?reg,?reg` must match `ld a, a` but never `ld a, c` - this is the
    // whole point of binding a capture once and requiring equality after.
    let found = matches_for(" ld a, a\n", LD_SELF);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].replacement.is_empty(), "rule deletes the instruction");

    assert!(matches_for(" ld a, c\n", LD_SELF).is_empty());
}

#[test]
fn the_in_constraint_actually_filters() {
    // `ld ixh, ixh` has exactly the shape the pattern wants (same register in
    // both slots) and really assembles, but `IXH` is not in the constraint's
    // candidate list, so the rule must not fire.
    let found = matches_for(" ld ixh, ixh\n", LD_SELF);
    assert!(found.is_empty(), "{found:?}");
    // ... whereas a register that *is* listed does fire, proving the previous
    // assertion is about the constraint rather than about the shape.
    assert_eq!(matches_for(" ld b, b\n", LD_SELF).len(), 1);
}

#[test]
fn a_rule_whose_constraints_are_unsupported_is_skipped_entirely() {
    // Same rule as CP_ZERO but carrying a constraint this crate cannot
    // evaluate. Skipping is the only safe behavior: matching anyway would
    // suggest an optimization whose safety condition was never checked.
    //
    // The name here is invented, because every constraint the upstream format
    // documents is now implemented. That is precisely why the test still
    // earns its place: it guards the mechanism protecting us from a constraint
    // a *future* upstream release adds, which would otherwise be ignored while
    // the rule was applied anyway. (This test had to be re-pointed three times
    // as each constraint it named got implemented - each time it kept passing
    // for a different reason than its name claimed, which is why the
    // counterpart test below exists.)
    let with_unsupported = "\
pattern: Replace cp 0 with or a
name: cp02ora
0: cp 0
replacement:
0: or a
constraints:
constraintFromAFutureRelease(0,A)
";
    assert!(matches_for(" cp 0\n", with_unsupported).is_empty());
    // ... while the same source does match once that constraint is gone.
    assert_eq!(matches_for(" cp 0\n", CP_ZERO).len(), 1);
}

/// The counterpart to the test above, and the thing that keeps it honest: a
/// rule carrying `flagsNotUsedAfter` must now be *evaluated* rather than
/// skipped, so the same rule can come out either way depending on what
/// follows the match.
#[test]
fn a_liveness_constraint_is_really_evaluated_rather_than_skipped() {
    let cp_zero_guarded = "\
pattern: Replace cp 0 with or a
name: cp02ora
0: cp 0
replacement:
0: or a
constraints:
flagsNotUsedAfter(0,N,P/V)
";
    // `xor a` writes every flag, so N and P/V are dead right after the `cp`:
    // the constraint is satisfied and the rule fires. Were the constraint
    // merely being skipped, this would be empty.
    let found = matches_for(" cp 0\n xor a\n ret\n", cp_zero_guarded);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].rule_name.as_deref(), Some("cp02ora"));

    // `ret pe` reads P/V before anything overwrites it, so the flag is live
    // and the rule must not fire - the same rule, opposite answer, decided by
    // the code that follows it.
    assert!(matches_for(" cp 0\n ret pe\n", cp_zero_guarded).is_empty());
}

#[test]
fn a_multi_instruction_pattern_matches_consecutive_instructions() {
    // The real upstream `regpair-transfer` rule: push/pop through a register
    // pair becomes two 8-bit loads. Exercises multi-line matching, `regpair`,
    // and two independent `in(...)` constraints at once.
    let rules = "\
pattern: Replace push ?regpair1; pop ?regpair2 with ld ?reg2h,?reg1h; ld ?reg2l,?reg1l
name: regpair-transfer
0: push ?regpair1
1: pop ?regpair2
replacement:
0: ld ?reg2h,?reg1h
1: ld ?reg2l,?reg1l
constraints:
in(?regpair1,BC,DE,HL)
in(?regpair2,BC,DE,HL)
regpair(?regpair1,?reg1h,?reg1l)
regpair(?regpair2,?reg2h,?reg2l)
";
    let found = matches_for(" push hl\n pop de\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range(), 0..2);
    // `regpair` binds the half-register names, which the replacement then
    // renders - proving constraints can feed captures back into the output.
    assert_eq!(found[0].replacement, vec![
        "ld d, h".to_string(),
        "ld e, l".to_string()
    ]);
}

#[test]
fn a_multi_instruction_pattern_does_not_match_across_a_gap() {
    let rules = "\
pattern: push then pop
0: push ?regpair1
1: pop ?regpair2
replacement:
0: nop
constraints:
in(?regpair1,BC,DE,HL)
in(?regpair2,BC,DE,HL)
";
    // An unrelated instruction between them breaks the required contiguity.
    assert!(matches_for(" push hl\n nop\n pop de\n", rules).is_empty());
}

#[test]
fn an_op_variable_binds_the_mnemonic_and_requires_consistency() {
    let rules = "\
pattern: Remove redundant ?op ?any
name: redundant-op
0: ?op ?any
1: ?op ?any
replacement:
0: ?op ?any
constraints:
in(?op,and,or)
";
    // Same opcode and same operand twice - the rule fires.
    let found = matches_for(" and b\n and b\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement, vec!["and b".to_string()]);

    // Different opcodes: `?op` is bound by the first line, so the second
    // cannot rebind it.
    assert!(matches_for(" and b\n or b\n", rules).is_empty());
    // Same opcode, different operand: `?any` is likewise already bound.
    assert!(matches_for(" and b\n and c\n", rules).is_empty());
    // An opcode outside the constraint's list.
    assert!(matches_for(" xor b\n xor b\n", rules).is_empty());
}

#[test]
fn a_symbolic_operand_keeps_its_original_spelling_in_the_replacement() {
    // The single most important property of the whole feature: a suggested
    // rewrite must never resolve a label to a literal address, or applying it
    // would silently freeze an address that moves the next time the file is
    // edited.
    let rules = "\
pattern: Replace jp ?const1 with jr ?const1
name: jp2jr
0: jp ?const1
replacement:
0: jr ?const1
";
    let found = matches_for("some_label:\n jp some_label\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement, vec!["jr some_label".to_string()]);
    assert!(
        found[0].replacement[0].contains("some_label"),
        "the label's own text must survive into the replacement"
    );
}

#[test]
fn a_mixed_case_label_survives_while_registers_are_canonicalised() {
    // The two halves of the case rule at once: a register capture is a
    // keyword and may be case-folded, a label capture must not be. Getting
    // this wrong in the "fold everything" direction silently rewrites
    // `SomeLabel` into a different symbol.
    let rules = "\
pattern: Replace jp ?const1 with jr ?const1
0: jp ?const1
replacement:
0: jr ?const1
";
    let found = matches_for("SomeLabel:\n JP SomeLabel\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement, vec!["jr SomeLabel".to_string()]);

    let reg_rules = "\
pattern: Remove ld ?reg,?reg
0: ld ?reg,?reg
replacement:
0: xor ?reg
constraints:
in(?reg,A,B,C,D,E,H,L)
";
    // Source written in upper case; the register renders canonically lower.
    let found = matches_for(" LD B, B\n", reg_rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement, vec!["xor b".to_string()]);
}

#[test]
fn the_message_substitutes_captured_variables() {
    let rules = "\
pattern: Remove ld ?reg,?reg
0: ld ?reg,?reg
replacement:
constraints:
in(?reg,A,B,C,D,E,H,L)
";
    let found = matches_for(" ld b, b\n", rules);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].message, "Remove ld b,b");
}

#[test]
fn matches_are_reported_for_every_occurrence_and_do_not_overlap() {
    let found = matches_for(" cp 0\n nop\n cp 0\n", CP_ZERO);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0].range(), 0..1);
    assert_eq!(found[1].range(), 2..3);
}

#[test]
fn a_wildcard_can_span_intervening_instructions() {
    let rules = "\
pattern: wildcard spanning
0: push ?regpair1
1: *
2: pop ?regpair2
replacement:
0: nop
constraints:
in(?regpair1,BC,DE,HL)
in(?regpair2,BC,DE,HL)
";
    let found = matches_for(" push hl\n nop\n nop\n pop de\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range(), 0..4);
    // The anchor is line 0's instruction, i.e. the `push`.
    assert_eq!(found[0].anchor, 0);
}

#[test]
fn a_fixed_repeat_matches_exactly_that_many_instructions() {
    let rules = "\
pattern: three nops
0: [3] nop
replacement:
0: nop
";
    let found = matches_for(" nop\n nop\n nop\n ret\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range(), 0..3);

    // Only two available - the fixed count cannot be satisfied.
    assert!(matches_for(" nop\n nop\n ret\n", rules).is_empty());
}

#[test]
fn an_empty_token_stream_yields_no_matches() {
    let rules = RuleSet::parse(CP_ZERO).unwrap();
    let tokens: Vec<&cpclib_asm::parser::LocatedToken> = Vec::new();
    assert!(find_matches(&tokens, &rules).is_empty());
}

#[test]
fn a_variable_repeat_binds_the_count_and_prefers_the_longest_run() {
    // `[?const1]` is a *variable* repeat: the matcher has to work out how many
    // consecutive instructions to consume and bind that number to `?const1`,
    // which the replacement then renders. Distinct from the fixed-count case
    // above, and the case the run-length optimisation in `match_lines` had to
    // preserve exactly - it must still take the longest run, not the first
    // one that happens to fit.
    let rules = "\
pattern: Collapse ?const1 shifts
name: collapse-shifts
0: [?const1] srl a
replacement:
0: [?const1] rrca
";
    let found = matches_for(" srl a\n srl a\n srl a\n ret\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range(), 0..3, "must consume all three, not fewer");
    assert_eq!(found[0].replacement, vec!["rrca : rrca : rrca".to_string()]);

    // A single occurrence is still a run of one.
    let found = matches_for(" srl a\n ret\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range(), 0..1);

    // The run stops at the first instruction that does not match, rather than
    // skipping over it.
    let found = matches_for(" srl a\n srl a\n nop\n srl a\n ret\n", rules);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0].range(), 0..2);
    assert_eq!(found[1].range(), 3..4);

    // Nothing to match at all.
    assert!(matches_for(" nop\n ret\n", rules).is_empty());
}

#[test]
fn a_replacement_that_cannot_be_rendered_declines_the_match() {
    // An empty replacement is not a failure signal - it is how a rule says
    // "delete these instructions" (see `LD_SELF` above, whose replacement is
    // genuinely empty). So a rule whose replacement lines cannot be rendered
    // must not fall back to producing an empty one: that would silently turn
    // "this rewrite cannot be expressed" into "delete the user's code".
    //
    // `and #ff >> ?const1` is a real upstream replacement line involving
    // arithmetic on a captured repeat count, which the renderer does not
    // handle. The rule must therefore report nothing at all.
    let unrenderable = "\
pattern: Collapse shifts
0: [?const1] srl a
replacement:
0: and #ff >> ?const1
";
    let found = matches_for(" srl a\n srl a\n srl a\n ret\n", unrenderable);
    assert!(
        found.is_empty(),
        "an unrenderable replacement must decline, not propose a deletion: {found:?}"
    );

    // Same pattern, same source, a replacement that *can* be rendered - so
    // the assertion above is about the rendering rather than about the
    // pattern failing to match in the first place.
    let renderable = "\
pattern: Collapse shifts
0: [?const1] srl a
replacement:
0: [?const1] rrca
";
    assert_eq!(matches_for(" srl a\n srl a\n srl a\n ret\n", renderable).len(), 1);
}
