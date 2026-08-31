//! `atLeastOneCPUOp`, `evenPushPopsSPNotRead`, `memoryNotWritten`,
//! `memoryNotUsed` and `noStackArguments` - the constraints that close out the
//! corpus.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};

/// `unnecessary-push-pop`, verbatim from `vendor/pbo-patterns.txt`.
const PUSH_POP: &str = "\
pattern: Remove push ?reg / pop ?reg as register is not modified in between
name: unnecessary-push-pop
0: push ?regpair
1: *
2: pop ?regpair
replacement:
1: *
constraints:
in(?regpair,BC,DE,HL,IX,IY)
regsNotModified(1,?regpair)
atLeastOneCPUOp(1)
evenPushPopsSPNotRead(1)
";

fn matches(source: &str, rules: &str) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("test source must parse");
    let rules = RuleSet::parse(rules).expect("test rules must parse");
    let tokens: Vec<_> = listing.iter().collect();
    find_matches(&tokens, &rules)
}

#[test]
fn a_push_pop_pair_around_untouched_work_is_removable() {
    let found = matches(
        "start:\n    push hl\n    ld a, 1\n    inc a\n    pop hl\n    ret\n",
        PUSH_POP
    );
    assert_eq!(found.len(), 1, "{found:?}");
    // The gap survives, one entry per instruction; only the push and pop go.
    assert_eq!(found[0].replacement.len(), 2, "{:?}", found[0].replacement);
    let kept = found[0].replacement.join(" | ").to_lowercase();
    assert!(kept.contains("ld a") && kept.contains("inc a"), "{kept:?}");
}

#[test]
fn a_push_pop_pair_whose_register_is_modified_stays() {
    assert!(
        matches(
            "start:\n    push hl\n    inc hl\n    pop hl\n    ret\n",
            PUSH_POP
        )
        .is_empty(),
        "HL is modified in between, so the pair is doing real work"
    );
}

/// The whole reason `atLeastOneCPUOp` exists, in upstream's own words: *"to
/// prevent eliminating the usual push af; pop af combination used for timing"*.
#[test]
fn an_empty_push_pop_pair_is_left_alone_because_it_is_timing_padding() {
    assert!(
        matches("start:\n    push hl\n    pop hl\n    ret\n", PUSH_POP).is_empty(),
        "an empty push/pop pair is cycle padding, not dead code"
    );
}

#[test]
fn an_unbalanced_gap_blocks_removal() {
    // The gap pushes without popping, so the `pop hl` is not retrieving what
    // the `push hl` stored - removing the pair would take the wrong value.
    assert!(
        matches(
            "start:\n    push hl\n    push bc\n    ld a, 1\n    pop hl\n    ret\n",
            PUSH_POP
        )
        .is_empty(),
        "the gap leaves the stack one entry deeper"
    );
}

#[test]
fn a_gap_that_touches_sp_directly_blocks_removal() {
    // `add hl, sp` observes the stack depth, which removing the surrounding
    // push/pop would change.
    assert!(
        matches(
            "start:\n    push de\n    ld hl, 0\n    add hl, sp\n    pop de\n    ret\n",
            PUSH_POP
        )
        .is_empty(),
        "the gap reads SP"
    );
}

#[test]
fn a_call_in_the_gap_blocks_removal() {
    // A call moves SP for reasons the balance count does not model.
    assert!(
        matches(
            "start:\n    push hl\n    call routine\n    pop hl\n    ret\nroutine:\n    ret\n",
            PUSH_POP
        )
        .is_empty(),
        "a call in the gap uses the stack"
    );
}

// ---------------------------------------------------------------------------
// noStackArguments
// ---------------------------------------------------------------------------

/// `tail-recursion`, verbatim: `call X; ret` becomes `jp X`.
const TAIL_RECURSION: &str = "\
pattern: Replace call ?const; ret with jp ?const
name: tail-recursion
tags: sdcc-unsafe
0: call ?const
1: ret
replacement:
0: jp ?const
constraints:
noStackArguments(?const)
";

#[test]
fn a_tail_call_to_a_plain_routine_becomes_a_jump() {
    let found = matches(
        "start:\n    call routine\n    ret\nroutine:\n    ld a, 1\n    ret\n",
        TAIL_RECURSION
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].replacement, vec!["jp routine".to_string()]);
}

#[test]
fn a_tail_call_to_a_routine_reading_inline_parameters_is_left_alone() {
    // The CPC idiom this constraint is really protecting: the routine pops its
    // own return address to read the bytes placed after the call site. Rewrite
    // `call X; ret` to `jp X` and that pop yields the *caller's* return
    // address instead, and the routine reads whatever follows that.
    assert!(
        matches(
            "start:\n    call routine\n    ret\nroutine:\n    pop hl\n    ld a, (hl)\n    ret\n",
            TAIL_RECURSION
        )
        .is_empty(),
        "the routine takes its argument off the stack"
    );
}

#[test]
fn a_tail_call_to_an_unanalysable_routine_is_left_alone() {
    // The routine jumps elsewhere, so this scan never reaches a `ret` it can
    // reason about - undecidable, therefore refused.
    assert!(
        matches(
            "start:\n    call routine\n    ret\nroutine:\n    jp elsewhere\nelsewhere:\n    ret\n",
            TAIL_RECURSION
        )
        .is_empty(),
        "the callee's stack behaviour cannot be determined"
    );
}

#[test]
fn a_tail_call_to_an_unknown_routine_is_left_alone() {
    assert!(
        matches("start:\n    call somewhere_else\n    ret\n", TAIL_RECURSION).is_empty(),
        "the callee is not in this listing at all"
    );
}

// ---------------------------------------------------------------------------
// memoryNotWritten / memoryNotUsed
// ---------------------------------------------------------------------------

/// `sdcc-inefficient-index-register-use1`, verbatim from the corpus - the only
/// place `memoryNotWritten` is used.
const MEM_GAP: &str = "\
pattern: Reuse an already-loaded indexed value
name: sdcc-inefficient-index-register-use1
2: ld ?reg1, (?regixiy + ?const1)
1: *
0: ld ?reg2, (?regixiy + ?const1)
replacement:
3: ld ?reg2, (?regixiy + ?const1)
2: ld ?reg1, ?reg2
1: *
constraints:
in(?regixiy,ix,iy)
regsNotUsed(1,?reg2)
regsNotModified(1,?reg2,?regixiy)
memoryNotWritten(1,?regixiy + ?const1)
";

#[test]
fn a_gap_that_writes_memory_blocks_the_rule() {
    // Deliberately stricter than upstream, which matches addresses
    // syntactically and would consider `(hl)` distinct from `(ix+4)`. Since
    // `hl` may well point there, the honest answer is "cannot tell".
    assert!(
        matches(
            "start:\n    ld a, (ix + 4)\n    ld (hl), 9\n    ld b, (ix + 4)\n    ret\n",
            MEM_GAP
        )
        .is_empty(),
        "a write through (hl) might alias (ix+4)"
    );
}

#[test]
fn a_gap_that_touches_no_memory_satisfies_the_constraint() {
    let found = matches(
        "start:\n    ld a, (ix + 4)\n    inc c\n    ld b, (ix + 4)\n    ret\n",
        MEM_GAP
    );
    assert_eq!(found.len(), 1, "{found:?}");
}

// ---------------------------------------------------------------------------
// Fidelity of a preserved region
// ---------------------------------------------------------------------------

#[test]
fn a_preserved_region_keeps_the_users_case_number_base_and_comments() {
    // The instructions between the push and the pop are not being changed by
    // this rule, so what comes back out must be what the user wrote - not a
    // canonical re-rendering of it. Rewriting `#7F10` as `0x7f10`, upper case
    // as lower, or dropping the comment would all be gratuitous edits to lines
    // the rule never touched.
    let found = matches(
        "start:\n    PUSH HL\n    LD BC, #7F10   ; set the gate array\n\
             OUT (C), C\n    POP HL\n    ret\n",
        PUSH_POP
    );
    assert_eq!(found.len(), 1, "{found:?}");
    let out = &found[0].replacement;

    assert!(
        out.iter().any(|l| l.contains("#7F10")),
        "the number's own base and case must survive: {out:?}"
    );
    assert!(
        out.iter().any(|l| l.contains("LD BC")),
        "the instruction's own case must survive: {out:?}"
    );
    assert!(
        out.iter().any(|l| l.contains("set the gate array")),
        "the comment must survive: {out:?}"
    );

    // A comment runs to end of line, so it must occupy an entry of its own -
    // folded onto the same line as a following instruction it would swallow it.
    let comment = out
        .iter()
        .position(|l| l.contains("set the gate array"))
        .expect("comment present");
    assert!(
        out[comment].trim_start().starts_with(';'),
        "the comment must stand alone on its line: {out:?}"
    );
    assert!(
        out.iter().any(|l| l.to_uppercase().contains("OUT (C), C")),
        "the instruction after the comment must not be swallowed by it: {out:?}"
    );
}
