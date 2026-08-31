//! Adversarial tests for the forward-liveness constraints
//! (`regsNotUsedAfter`/`flagsNotUsedAfter`), driven through the *real* upstream
//! rules they unlocked rather than through the walker's own API.
//!
//! The walker has its own unit tests; this file exists for a different reason.
//! A liveness constraint is the only thing standing between "this instruction
//! looks dead" and actually deleting code from a user's demo, so the failure
//! that matters is not "the walker returned the wrong enum" - it is "the rule
//! fired when it should not have". That property is only observable end to
//! end, which is what every case here checks.
//!
//! Every test therefore comes in pairs: an adversarial source that must produce
//! **no** match, and a near-identical control that must produce one. Without
//! the control, a test asserting emptiness would still pass if the rule had
//! silently stopped working altogether - which is precisely the regression a
//! fail-closed design is most likely to hide.

use cpclib_asm::parser::parse_z80_str;
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::{PeepholeMatch, find_matches};

/// `unused-ld-any`, copied verbatim from `vendor/pbo-patterns.txt`: delete a
/// load whose destination register is never read afterwards.
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

/// `cp12deca`, verbatim: `cp 1` becomes `dec a`, but only when both A and the
/// carry flag are dead afterwards - `dec a` does not set carry the way `cp 1`
/// does, which is exactly what the flag half of the constraint protects.
const CP1_TO_DEC_A: &str = "\
pattern: Replace cp 1 with dec a
name: cp12deca
0: cp 1
replacement:
0: dec a
constraints:
regsNotUsedAfter(0,A)
flagsNotUsedAfter(0,C)
";

fn matches(source: &str, rules: &str) -> Vec<PeepholeMatch> {
    let listing = parse_z80_str(source).expect("test source must parse");
    let rules = RuleSet::parse(rules).expect("test rules must parse");
    let tokens: Vec<_> = listing.iter().collect();
    find_matches(&tokens, &rules)
}

fn assert_no_match(source: &str, rules: &str, why: &str) {
    let found = matches(source, rules);
    assert!(found.is_empty(), "must not fire ({why}): {found:?}");
}

/// `anchor` is the 0-based index, in the token stream, of the instruction the
/// match must be reported on. Pinning it down matters: these sources contain
/// several instructions the same rule *could* match, so a bare count of 1
/// would still pass if the rule had fired on the wrong one - at which point
/// the "control" would no longer be controlling for anything.
fn assert_fires(source: &str, rules: &str, anchor: usize, why: &str) {
    let found = matches(source, rules);
    assert_eq!(found.len(), 1, "should fire ({why}): {found:?}");
    assert_eq!(
        found[0].anchor, anchor,
        "fired on the wrong instruction ({why}): {found:?}"
    );
}

// ---------------------------------------------------------------------------
// CALL - followed into the callee, not treated as an opaque barrier
// ---------------------------------------------------------------------------

#[test]
fn a_call_whose_callee_reads_the_register_blocks_the_rule() {
    // The read is inside the callee, never in the straight-line code after
    // the match. A walker that stopped at `call` - or that skipped over it to
    // the next instruction - would happily delete a live load.
    assert_no_match(
        "start:\n    ld b, 1\n    call routine\n    ret\nroutine:\n    ld a, b\n    ret\n",
        UNUSED_LD,
        "the callee reads B"
    );
}

#[test]
fn a_call_whose_callee_overwrites_the_register_still_allows_the_rule() {
    // Control for the case above: same shape, but the callee *writes* B
    // instead of reading it, so the load really is dead. This is what proves
    // the previous test is about the read rather than about `call` being
    // treated as unanalysable.
    assert_fires(
        "start:\n    ld b, 1\n    call routine\n    ld a, b\n    ret\nroutine:\n    ld b, 9\n    ret\n",
        UNUSED_LD,
        1,
        "the callee overwrites B before anything reads it"
    );
}

// ---------------------------------------------------------------------------
// RET - an unmatched return is "unclear", not "end of story"
// ---------------------------------------------------------------------------

#[test]
fn a_ret_reached_without_a_tracked_call_is_unclear_and_fails_closed() {
    // Execution leaves through a `ret` whose caller we cannot see, so the
    // register may well be this routine's return value. Answering "not used"
    // here would delete a function's result.
    assert_no_match(
        "start:\n    ld b, 1\n    ret\n",
        UNUSED_LD,
        "B may be the routine's return value"
    );
}

// ---------------------------------------------------------------------------
// Loops - the case a naive walker either misses or hangs on
// ---------------------------------------------------------------------------

#[test]
fn a_loop_that_reads_the_register_blocks_the_rule() {
    // `djnz` reads B, and the only path to it goes around a backward jump.
    // A forward-only scan would never see this read.
    assert_no_match(
        "start:\n    ld b, 1\nloop:\n    nop\n    djnz loop\n    ld c, 2\n    ld a, c\n    ret\n",
        UNUSED_LD,
        "djnz reads B"
    );
}

#[test]
fn a_loop_that_never_touches_the_register_still_terminates_and_allows_the_rule() {
    // The other half of the loop story, and the one that would expose a
    // missing closed-set: the walker has to go round the loop, notice it has
    // already been there with the same dependency, and carry on to the kill
    // rather than looping forever or giving up with "unclear".
    assert_fires(
        "start:\n    ld b, 1\nloop:\n    dec c\n    jr nz, loop\n    ld b, 5\n    ld a, b\n    ret\n",
        UNUSED_LD,
        1,
        "the loop never touches B and B is overwritten after it"
    );
}

// ---------------------------------------------------------------------------
// Jumps whose target cannot be resolved
// ---------------------------------------------------------------------------

#[test]
fn a_computed_jump_is_unclear_and_fails_closed() {
    // `jp (hl)` can go anywhere; nothing downstream is knowable.
    assert_no_match(
        "start:\n    ld b, 1\n    jp (hl)\n",
        UNUSED_LD,
        "jp (hl) has an unknowable target"
    );
}

#[test]
fn a_conditional_jump_must_have_both_arms_clear_not_just_one() {
    // The fallthrough arm overwrites B, so a walker that only followed one
    // successor could conclude "dead". The taken arm reads it.
    assert_no_match(
        "start:\n    ld b, 1\n    jr z, elsewhere\n    ld b, 2\n    ld a, b\n    ret\n\
         elsewhere:\n    ld a, b\n    ret\n",
        UNUSED_LD,
        "the taken branch reads B even though the fallthrough kills it"
    );
}

#[test]
fn running_off_the_end_of_the_listing_is_unclear_and_fails_closed() {
    // No terminator at all: execution simply runs past what we can see.
    assert_no_match(
        "start:\n    ld b, 1\n    nop\n",
        UNUSED_LD,
        "execution continues past the end of what we can see"
    );
}

// ---------------------------------------------------------------------------
// Flags - the same walk, tracking something that is never an operand
// ---------------------------------------------------------------------------

#[test]
fn a_later_flag_read_blocks_a_rule_even_when_the_registers_are_dead() {
    // A is dead in both, so only the *flag* half of `cp12deca`'s constraint
    // can distinguish them - `ret c` reads carry, `ret` does not.
    assert_no_match(
        "start:\n    cp 1\n    ret c\n    xor a\n    ld a, 0\n    ret\n",
        CP1_TO_DEC_A,
        "ret c reads the carry flag that cp 1 sets"
    );
}

#[test]
fn a_dead_flag_and_a_dead_register_together_allow_the_rule() {
    // Control: `xor a` rewrites A and every flag, killing both halves of the
    // constraint at once, so the rewrite is genuinely safe here.
    assert_fires(
        "start:\n    cp 1\n    xor a\n    ld (hl), a\n    ret\n",
        CP1_TO_DEC_A,
        1,
        "xor a kills both A and the carry flag"
    );
}

// ---------------------------------------------------------------------------
// Fake instructions in the walk path
// ---------------------------------------------------------------------------
//
// basm lets the user write `ld de, hl`, which is not a real Z80 instruction -
// it assembles to `ld d,h : ld e,l`. The matcher works on tokens, so it never
// splits one of these; the *walker* however has to see through it, because the
// registers an expansion touches are invisible on the token itself. There is
// no instruction-set row for `ld de,hl`, so a walker that did not expand would
// have no choice but to answer "unclear" - which makes the first test below a
// genuine discriminator rather than a formality.

#[test]
fn a_fake_instructions_expansion_can_kill_the_dependency() {
    // `ld de, hl` expands to `ld d,h : ld e,l`; the first step writes D, so
    // the earlier `ld d, 1` really is dead. Answering this correctly is only
    // possible by looking inside the expansion.
    assert_fires(
        "start:\n    ld d, 1\n    ld de, hl\n    ld a, d\n    ret\n",
        UNUSED_LD,
        1,
        "the expansion of ld de,hl writes D"
    );
}

#[test]
fn a_fake_instructions_expansion_can_read_the_dependency() {
    // The mirror image: the same expansion *reads* H, so a load into H before
    // it is live and must be kept.
    assert_no_match(
        "start:\n    ld h, 1\n    ld de, hl\n    ld h, 9\n    ld a, h\n    ret\n",
        UNUSED_LD,
        "the expansion of ld de,hl reads H"
    );
}

#[test]
fn a_match_on_a_fake_instruction_is_reported_on_the_token_the_user_wrote() {
    // The rule matches the fake instruction itself. Its expansion occupies
    // several slots in the analysis stream, but the reported anchor must be
    // the single token as it appears in the user's file - reporting a
    // diagnostic at a position nobody wrote would be unusable in the editor.
    assert_fires(
        "start:\n    ld de, hl\n    ld de, 5\n    ld a, e\n    ret\n",
        UNUSED_LD,
        1,
        "DE is overwritten right after the fake instruction"
    );
}

// ---------------------------------------------------------------------------
// Multi-register push/pop
// ---------------------------------------------------------------------------

#[test]
fn a_multi_register_push_is_seen_as_the_pushes_it_stands_for() {
    // basm lets several registers share one `push`. It carries no mnemonic of
    // its own, so before it was expanded it looked structurally just like a
    // label - inert - and the walk stepped straight over it without noticing
    // it reads B.
    //
    // Taken from real code (a sprite blitter in `birthtro`), where that made
    // the optimizer offer to delete the `ld b` driving the outer loop.
    assert_no_match(
        "start:\n    ld b, 8\n.loopy\n    push bc, hl\n    ld b, 4\n    ld a, (de)\n    ret\n",
        UNUSED_LD,
        "push bc, hl reads B"
    );

    // The control that makes the above about the *expansion* rather than
    // about multi-push being opaque: pushing registers that have nothing to
    // do with B leaves the load genuinely dead, and the rule fires. That can
    // only be decided by looking at which registers the statement expands to.
    assert_fires(
        "start:\n    ld b, 8\n.loopy\n    push de, hl\n    ld b, 4\n    ld a, (de)\n    ret\n",
        UNUSED_LD,
        1,
        "push de, hl touches neither B nor anything else that keeps it alive"
    );
}

#[test]
fn a_multi_register_pop_is_seen_as_the_pops_it_stands_for() {
    // The mirror case: `pop bc, hl` *writes* B, so it kills the dependency
    // and the earlier load really is dead.
    assert_fires(
        "start:\n    ld b, 8\n    pop bc, hl\n    ld a, b\n    ret\n",
        UNUSED_LD,
        1,
        "pop bc, hl overwrites B"
    );
}

// ---------------------------------------------------------------------------
// The stack pointer
// ---------------------------------------------------------------------------

#[test]
fn the_stack_pointer_is_never_reported_as_dead() {
    // Nothing in this listing reads SP, so a purely syntactic forward walk
    // concludes the `ld sp` is dead. It is not: an interrupt can fire between
    // any two instructions and pushes onto the stack, and on a CPC the
    // firmware runs on interrupts by default.
    //
    // Real sequence, from `birthtro` - setting up the stack and then parking
    // in a frame loop.
    assert_no_match(
        "start:\n    ld sp, $\nframe:\n    jp frame\n",
        UNUSED_LD,
        "SP is read by interrupt handling that appears nowhere in the listing"
    );

    // A different register in exactly the same position is still reported, so
    // the assertion above is about SP specifically rather than about the
    // frame loop defeating the walk.
    assert_fires(
        "start:\n    ld hl, $\n    ld hl, 5\n    ld a, l\n    ret\n",
        UNUSED_LD,
        1,
        "HL carries no such architectural liveness"
    );
}

// ---------------------------------------------------------------------------
// Self-modifying code
// ---------------------------------------------------------------------------

/// `ld0-to-xor`, verbatim: `ld a,0` becomes `xor a`. Two bytes become one, so
/// anything pointing into the instruction's encoding breaks.
const LD0_TO_XOR: &str = "\
pattern: Replace ld a,0 with xor a
name: ld0-to-xor
0: ld a,0
replacement:
0: xor a
constraints:
flagsNotUsedAfter(0,S,Z,H,P/V,N,C)
";

#[test]
fn an_instruction_named_by_an_equ_offset_is_not_rewritten() {
    // The `equ $-1` names the immediate operand byte, so shrinking `ld a,0`
    // to `xor a` leaves `.activated` pointing at whatever follows. Nothing
    // about that fails at assembly time.
    assert_no_match(
        "start:\n    ld a, 0 : .activated equ $-1\n    xor a\n    ret\n",
        LD0_TO_XOR,
        ".activated points into the ld's encoding"
    );

    // Control: the identical instruction and identical following code, minus
    // the equ. The rule fires, so the assertion above is about the equ rather
    // than about the rule being unable to match here.
    assert_fires(
        "start:\n    ld a, 0\n    xor a\n    ret\n",
        LD0_TO_XOR,
        1,
        "nothing points into this ld"
    );
}

#[test]
fn an_instruction_whose_label_is_used_with_an_offset_is_not_rewritten() {
    // The other common idiom: the patch site is reached as `label+1`.
    assert_no_match(
        "phase_\n    ld a, 0\n    ld (phase_+1), a\n    xor a\n    ret\n",
        LD0_TO_XOR,
        "phase_+1 points into the ld's encoding"
    );
}
