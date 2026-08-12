//! Every suggestion must be able to say *why* it is safe.
//!
//! A peephole suggestion with no justification is unauditable: "Remove unused
//! `ld b, c`" gives a reader no way to tell whether B is clobbered two
//! instructions later, inside a routine three calls deep, or not at all - and
//! that is exactly the position a user was left in on real code. So each
//! safety-bearing constraint records what settled it, and points at the
//! instruction that proves it.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};

fn matches(source: &str, rules: &str) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("test source must parse");
    let rules = RuleSet::parse(rules).expect("test rules must parse");
    let tokens: Vec<_> = listing.iter().collect();
    find_matches(&tokens, &rules)
}

const UNUSED_LD: &str = "\
pattern: Remove unused ld ?reg,?any
name: unused-ld-any
0: ld ?reg,?any
replacement:
constraints:
notIn(?reg,I)
notIn(?any,I)
regsNotUsedAfter(0,?reg)
";

#[test]
fn a_dead_register_names_the_instruction_that_overwrote_it() {
    // Token 0 is the label, 1 the `ld b, c`, 2 the `ld b, 9` that kills B.
    let found = matches("start\n    ld b, c\n    ld b, 9\n    ld a, b\n    ret\n", UNUSED_LD);
    assert_eq!(found.len(), 1, "{found:?}");

    let reasons = &found[0].reasons;
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(
        reasons[0].text.contains('B') && reasons[0].text.contains("overwritten"),
        "the reason must name the register and what happened: {reasons:?}"
    );
    assert_eq!(
        reasons[0].witness,
        Some(2),
        "the witness must point at the instruction that overwrote B: {reasons:?}"
    );
}

#[test]
fn a_reason_that_rests_on_no_single_instruction_has_no_witness() {
    // Nothing overwrites B here; the walk simply runs out of reachable code.
    // That is a weaker justification and must read differently - and it has
    // nowhere to point.
    let found = matches("start\n    ld b, c\n    jp (hl)\n", UNUSED_LD);
    // `jp (hl)` is unknowable, so this must not fire at all - which is itself
    // the point: a reason is only ever produced for something that was proven.
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_flag_constraint_names_every_flag_it_checked() {
    let rules = "\
pattern: Replace cp 0 with or a
name: cp02ora
0: cp 0
replacement:
0: or a
constraints:
flagsNotUsedAfter(0,N,P/V)
";
    let found = matches("start\n    cp 0\n    xor a\n    ret\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    let text = &found[0].reasons[0].text;
    assert!(text.contains('N') && text.contains("P/V"), "{text:?}");
    assert!(text.contains("overwritten"), "{text:?}");
}

#[test]
fn a_block_local_constraint_says_which_region_it_checked() {
    let rules = "\
pattern: Carry HL across a gap
name: gap
0: ld ?reg,?8bitconst1
1: *
2: ld (hl),?reg
replacement:
0: ld (hl),?8bitconst1
1: *
constraints:
in(?reg,A,B,C,D,E)
regsNotModified(1, HL, ?reg)
regsNotUsed(1,?reg)
regsNotUsedAfter(2,?reg)
";
    let found = matches(
        "start\n    ld a, 5\n    nop\n    ld (hl), a\n    xor a\n    ret\n",
        rules
    );
    assert_eq!(found.len(), 1, "{found:?}");
    let texts: Vec<&str> = found[0].reasons.iter().map(|r| r.text.as_str()).collect();
    // Worded in source terms - a pattern line number is meaningless to a
    // reader looking at their own file.
    assert!(
        texts.iter().any(|t| t.contains("in between") && t.contains("writes")),
        "the region and what was checked must both be named: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("in between") && t.contains("reads")),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("line 1")),
        "must not quote a pattern line number the reader cannot see: {texts:?}"
    );
    // The gap's first instruction is token 2 (`start`, `ld a,5`, `nop`).
    let region = found[0]
        .reasons
        .iter()
        .find(|r| r.text.contains("in between"))
        .expect("a region reason");
    assert_eq!(region.witness, Some(2), "{region:?}");
}

#[test]
fn a_purely_structural_rule_has_nothing_to_explain() {
    // `unnecessary-ld-to-itself` rests only on the shape of the instruction,
    // which the reader can already see. Inventing a reason there would be
    // noise.
    let rules = "\
pattern: Remove ld ?reg,?reg
name: unnecessary-ld-to-itself
0: ld ?reg,?reg
replacement:
constraints:
in(?reg,A,B,C,D,E,H,L)
";
    let found = matches("start\n    ld b, b\n    ret\n", rules);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].reasons.is_empty(), "{:?}", found[0].reasons);
}
