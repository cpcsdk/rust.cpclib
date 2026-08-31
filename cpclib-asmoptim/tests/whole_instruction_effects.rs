//! `regFlagEffectsNotUsedAfter(#1, #2)`: every register and flag line `#1`
//! writes is dead after line `#2`.
//!
//! The four rules using it (`unnecessary-0args` .. `-2args-ex`) have an
//! *empty* replacement - they delete the instruction outright. That makes this
//! the most destructive constraint in the set, and the reason most of these
//! tests are about what it must refuse rather than what it allows.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};

/// `unnecessary-2args`, verbatim from `vendor/pbo-patterns.txt`.
const UNNECESSARY_2ARGS: &str = "\
pattern: remove unused ?op ?any1, ?any2
name: unnecessary-2args
0: ?op ?any1, ?any2
replacement:
constraints:
in(?op,adc,add,sbc,bit,set,res,ld)
regFlagEffectsNotUsedAfter(0, 0)
";

fn matches(source: &str) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("test source must parse");
    let rules = RuleSet::parse(UNNECESSARY_2ARGS).expect("test rules must parse");
    let tokens: Vec<_> = listing.iter().collect();
    find_matches(&tokens, &rules)
}

#[test]
fn an_instruction_whose_whole_output_is_dead_is_removable() {
    // `ld b, 1` writes only B, and `ld b, 2` overwrites it before anything
    // reads it. Nothing else the instruction produced can be observed.
    let found = matches("start:\n    ld b, 1\n    ld b, 2\n    ld a, b\n    ret\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].anchor, 1, "must be the first load: {found:?}");
    assert!(
        found[0].replacement.is_empty(),
        "this rule deletes the instruction"
    );
}

#[test]
fn a_live_register_keeps_the_instruction() {
    assert!(
        matches("start:\n    ld b, 1\n    ld a, b\n    ret\n").is_empty(),
        "B is read straight afterwards"
    );
}

#[test]
fn a_live_flag_keeps_the_instruction_even_when_the_registers_are_dead() {
    // `add a, b` writes A *and* all the flags. A is overwritten by `ld a, 0`,
    // but `ret c` reads carry first - so the instruction is not removable even
    // though its register output is dead. Only checking registers would delete
    // the very thing that sets up the branch.
    assert!(
        matches("start:\n    add a, b\n    ret c\n    ld a, 0\n    ret\n").is_empty(),
        "the carry flag it sets is read by `ret c`"
    );
}

#[test]
fn an_instruction_that_writes_memory_is_never_removable() {
    // The case that makes this constraint dangerous: `?op` includes `ld`, and
    // `ld (hl), a` leaves *every* register and flag dead while doing the only
    // thing it was written to do. Register liveness cannot speak to memory, so
    // the answer has to be "undecidable", not "unused".
    assert!(
        matches("start:\n    ld (hl), a\n    ret\n").is_empty(),
        "storing to memory is an effect no register liveness can prove dead"
    );

    // ... and with the store's registers unambiguously dead afterwards, to
    // show the refusal is about the memory write rather than about A or HL
    // still being live.
    assert!(
        matches("start:\n    ld (hl), a\n    ld a, 0\n    ld hl, 0\n    ret\n").is_empty(),
        "still a memory write, however dead A and HL are"
    );
}

#[test]
fn an_indexed_store_is_also_protected() {
    assert!(
        matches("start:\n    ld (ix + 4), a\n    ld a, 0\n    ret\n").is_empty(),
        "an indexed store writes memory just as much as (hl) does"
    );
}

#[test]
fn a_store_to_an_absolute_address_is_also_protected() {
    assert!(
        matches("buffer\n    defb 0\nstart:\n    ld (buffer), a\n    ld a, 0\n    ret\n").is_empty(),
        "writing to a named address is still a memory write"
    );
}
