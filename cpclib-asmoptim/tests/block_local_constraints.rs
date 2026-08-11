//! The block-local constraint family: `regsNotModified`, `regsNotUsed`,
//! `flagsNotModified`, `flagsNotUsed`.
//!
//! These ask what the instructions a pattern line *itself* matched do, with no
//! control-flow walk involved - and the line is usually a `*` wildcard, so the
//! real question is "does anything in this gap disturb what the rule wants to
//! carry across it?". That makes the region's *extent* load-bearing in a way
//! nothing else in the engine needed: a constraint that only looked at the
//! gap's first instruction would wave through a rule whose second instruction
//! clobbers the register.
//!
//! Every case here is a pair - a gap that must block the rule, and a
//! near-identical gap that must not - because a constraint that always
//! answered "blocked" would pass every negative test on its own.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};

/// `unnecessary-intermediate-reg`, verbatim from `vendor/pbo-patterns.txt`.
/// Uses three of the four constraints at once, over a `*` gap line.
const INTERMEDIATE_REG: &str = "\
pattern: Replace ld ?reg,?const1; ld (hl),?reg with ld (hl),?const1
name: unnecessary-intermediate-reg
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

/// Wraps `gap` in the surrounding code the rule needs: a load into A, the gap,
/// the store through HL, then `xor a` so A is provably dead afterwards (the
/// rule's own `regsNotUsedAfter(2,?reg)` would otherwise be undecidable and
/// every case here would pass for the wrong reason).
fn with_gap(gap: &str) -> String {
    format!("start:\n    ld a, 5\n{gap}    ld (hl), a\n    xor a\n    ret\n")
}

fn matches(source: &str, rules: &str) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("test source must parse");
    let rules = RuleSet::parse(rules).expect("test rules must parse");
    let tokens: Vec<_> = listing.iter().collect();
    find_matches(&tokens, &rules)
}

fn assert_blocked(gap: &str, why: &str) {
    let found = matches(&with_gap(gap), INTERMEDIATE_REG);
    assert!(found.is_empty(), "must not fire ({why}): {found:?}");
}

fn assert_fires(gap: &str, why: &str) {
    let found = matches(&with_gap(gap), INTERMEDIATE_REG);
    assert_eq!(found.len(), 1, "should fire ({why}): {found:?}");
    assert_eq!(
        found[0].rule_name.as_deref(),
        Some("unnecessary-intermediate-reg")
    );
}

#[test]
fn an_empty_gap_touches_nothing_and_the_rule_fires() {
    // The `*` matches zero instructions. Trivially "does not modify HL" - and
    // a region-based check that mishandled an empty range would either panic
    // or report unknown here, so this is the base case everything else varies
    // from.
    assert_fires("", "nothing between the load and the store");
}

#[test]
fn a_gap_that_leaves_everything_alone_still_fires() {
    assert_fires("    nop\n    nop\n", "nop touches neither HL nor A");
}

#[test]
fn a_gap_that_modifies_hl_blocks_the_rule() {
    // `ld (hl), 5` would write somewhere else entirely if HL moved in between.
    assert_blocked("    inc hl\n", "inc hl modifies HL");
}

#[test]
fn a_gap_that_modifies_the_carried_register_blocks_the_rule() {
    // `ld a, b` writes A without reading it, so `regsNotModified` is the only
    // constraint that can catch it - and unlike `ld a, 9` it is not itself a
    // load-of-a-constant the same rule would legitimately match one
    // instruction later.
    assert_blocked("    ld a, b\n", "the gap overwrites A");
}

#[test]
fn a_gap_that_only_reads_the_carried_register_still_blocks_the_rule() {
    // `ld b, a` does not *modify* A, so `regsNotModified` is satisfied - only
    // `regsNotUsed` catches this. Removing the intermediate load would leave
    // this read seeing whatever A held before.
    assert_blocked("    ld b, a\n", "the gap reads A even though it never writes it");
}

#[test]
fn only_the_second_instruction_of_a_gap_needs_to_disturb_it() {
    // The discriminating case for region *extent*: the first instruction of
    // the gap is harmless, so anything checking only where the region begins
    // would let this through.
    assert_blocked(
        "    nop\n    inc hl\n",
        "the gap's second instruction modifies HL"
    );
    // ...and with the order reversed, to show it is not simply "a two
    // instruction gap is always refused".
    assert_blocked(
        "    inc hl\n    nop\n",
        "the gap's first instruction modifies HL"
    );
    // Control: two instructions, neither disturbing anything.
    assert_fires("    nop\n    ld c, 1\n", "neither gap instruction touches HL or A");
}

#[test]
fn a_gap_containing_data_is_unknown_and_blocks_the_rule() {
    // Same fail-closed policy as the forward walk: a region whose contents
    // cannot all be described must never come back "leaves things alone".
    assert_blocked("    defb 0\n", "raw data in the gap cannot be reasoned about");
}

#[test]
fn a_multi_register_push_in_the_gap_is_read_through_rather_than_refused() {
    // `push bc, hl` carries no mnemonic of its own, but it *is* interpretable
    // - it expands to `push bc : push hl`, which read BC and HL without
    // writing either. So `regsNotModified(1, HL, ?reg)` is genuinely
    // satisfied and the rule fires, rather than being conservatively refused.
    assert_fires(
        "    push bc, hl\n",
        "pushes read HL but never write it, and never touch A"
    );
}

// ---------------------------------------------------------------------------
// flagsNotModified / flagsNotUsed
// ---------------------------------------------------------------------------

/// A cut-down rule exercising the flag half of the family over a `*` gap.
const FLAG_GAP: &str = "\
pattern: Carry a flag across a gap
name: flag-gap
0: cp 0
1: *
2: ret z
replacement:
0: or a
1: *
2: ret z
constraints:
flagsNotModified(1,Z)
";

#[test]
fn a_gap_that_modifies_the_carried_flag_blocks_the_rule() {
    let blocked = matches(" cp 0\n inc a\n ret z\n", FLAG_GAP);
    assert!(
        blocked.is_empty(),
        "inc a writes Z, so the flag cannot be carried across: {blocked:?}"
    );

    // `ld` does not touch the flags at all, so the same rule fires.
    let fires = matches(" cp 0\n ld b, 1\n ret z\n", FLAG_GAP);
    assert_eq!(fires.len(), 1, "ld leaves Z alone: {fires:?}");
    assert_eq!(fires[0].rule_name.as_deref(), Some("flag-gap"));
}

// ---------------------------------------------------------------------------
// The `*` line in a *replacement*
// ---------------------------------------------------------------------------

#[test]
fn a_wildcard_in_the_replacement_writes_the_gap_back_out() {
    // 118 upstream rules have a `*` in their replacement, meaning "and these
    // instructions stay as they are". A consumer replaces the whole matched
    // span with the replacement lines, so emitting nothing for that line would
    // silently *delete* the gap - the rule would keep its promise to preserve
    // those instructions by deleting them.
    let found = matches(&with_gap("    nop\n    ld c, 1\n"), INTERMEDIATE_REG);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement.len(), 2, "{:?}", found[0].replacement);
    assert_eq!(found[0].replacement[0], "ld (hl), 5");
    // The gap comes back, both instructions of it, in order.
    let gap = &found[0].replacement[1];
    assert!(gap.to_lowercase().contains("nop"), "{gap:?}");
    assert!(gap.to_lowercase().contains("ld c"), "{gap:?}");
}

#[test]
fn a_symbol_inside_a_preserved_gap_survives_verbatim() {
    // The bug class this codebase has been burned by more than once: case
    // folding a region that contains a real symbol silently retargets it.
    let found = matches(
        &with_gap("    call MyRoutine\n"),
        INTERMEDIATE_REG
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].replacement[1].contains("MyRoutine"),
        "the symbol's own spelling must survive: {:?}",
        found[0].replacement
    );
}

#[test]
fn an_empty_gap_contributes_no_replacement_line_at_all() {
    // Rather than a blank line, which would leave a stray empty line behind
    // when the fix is applied.
    let found = matches(&with_gap(""), INTERMEDIATE_REG);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement, vec!["ld (hl), 5".to_string()]);
}
