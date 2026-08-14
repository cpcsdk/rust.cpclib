//! Register/flag liveness over real parsed Z80 source. See `cost_range.rs` in
//! this directory for why these tests live outside the crate they exercise.

use cpclib_z80flow::dependency::Dependency;
use cpclib_z80flow::liveness::{Usage, is_used_after, label_index};
use cpclib_z80flow::regflag::{Flag, Reg};
use cpclib_z80flow::stream::build_without_addresses;
use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::{LocatedToken, parse_z80_str};


/// Walk from just after the instruction on 0-based `after_index` (counting
/// only real instructions, so tests read like the source).
fn usage(source: &str, after_index: usize, dep: Dependency) -> Usage {
    let listing = parse_z80_str(source).expect("source must parse");
    let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
    let stream = build_without_addresses(&tokens);
    let labels = label_index(&stream);

    let nth_instruction = stream
        .ops()
        .iter()
        .enumerate()
        .filter(|(_, op)| op.mnemonic().is_some())
        .map(|(i, _)| i)
        .nth(after_index)
        .expect("instruction index out of range");

    is_used_after(&stream, &labels, nth_instruction + 1, dep).usage
}

fn reg(r: Reg) -> Dependency {
    Dependency::Reg(r)
}

#[test]
fn a_value_read_by_the_next_instruction_is_used() {
    // ld a, 1 / ld b, a  -> A is read.
    assert_eq!(
        usage("    ld a, 1\n    ld b, a\n    ret\n", 0, reg(Reg::A)),
        Usage::Used
    );
}

#[test]
fn a_value_overwritten_before_any_read_is_not_used() {
    // ld a, 1 / ld a, 2 / ret -> the first A is dead.
    assert_eq!(
        usage("    ld a, 1\n    ld a, 2\n    ret\n", 0, reg(Reg::A)),
        Usage::NotUsed
    );
}

/// The narrowing model end to end: writing `B` leaves `C` live, so a
/// later read of `C` still counts as using `BC`.
#[test]
fn writing_one_half_leaves_the_other_half_live() {
    assert_eq!(
        usage("    ld bc, 0\n    ld b, 1\n    ld a, c\n    ret\n", 0, reg(Reg::Bc)),
        Usage::Used
    );
    // ...and once *both* halves are rewritten, the original is dead.
    assert_eq!(
        usage(
            "    ld bc, 0\n    ld b, 1\n    ld c, 2\n    ld a, c\n    ret\n",
            0,
            reg(Reg::Bc)
        ),
        Usage::NotUsed
    );
}

/// A `RET` this walk never entered via a `CALL` goes somewhere unknown.
#[test]
fn returning_to_an_unknown_caller_is_unknown() {
    assert_eq!(
        usage("    ld a, 1\n    ret\n", 0, reg(Reg::A)),
        Usage::Unknown
    );
}

/// Falling off the end of the file is not "the program ended" - the
/// stream is only what we can see.
#[test]
fn running_out_of_instructions_is_unknown() {
    assert_eq!(usage("    ld a, 1\n    nop\n", 0, reg(Reg::A)), Usage::Unknown);
}

/// The loop cases the memoized worklist exists for.
#[test]
fn a_loop_that_reads_the_value_reports_it_used() {
    let source = "\
ld a, 1
loop:
inc b
or a
jr nz, loop
ret
";
    // `A` is read by `or a` inside the loop.
    assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
}

#[test]
fn a_loop_that_never_touches_the_value_terminates_and_reports_not_used() {
    let source = "\
ld a, 1
loop:
inc b
dec c
jr nz, loop
ld a, 2
ret
";
    // Nothing in the loop reads `A`, and it is overwritten after it.
    assert_eq!(usage(source, 0, reg(Reg::A)), Usage::NotUsed);
}

/// Both arms of a conditional branch have to be explored - a read on
/// either side counts.
#[test]
fn both_arms_of_a_branch_are_explored() {
    let source = "\
ld a, 1
jr z, taken
ld a, 2
jr done
taken:
ld b, a
done:
ld a, 3
ret
";
    // The taken arm reads `A`; the fallthrough overwrites it.
    assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
}

/// A call is followed into and back out of.
#[test]
fn a_call_is_followed_and_returns_to_the_call_site() {
    // NB: the label can't be called `sub` - that's a real Z80 mnemonic.
    let source = "\
ld a, 1
call routine
ld b, a
ret
routine:
nop
ret
";
    // The subroutine doesn't touch `A`, but the instruction after the
    // call reads it - so the walk must come back.
    assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
}

#[test]
fn a_value_read_inside_a_called_subroutine_is_used() {
    let source = "\
ld a, 1
call routine
ret
routine:
ld b, a
ret
";
    assert_eq!(usage(source, 0, reg(Reg::A)), Usage::Used);
}

/// A computed jump has an unknowable continuation.
#[test]
fn a_computed_jump_is_unknown() {
    assert_eq!(
        usage("    ld a, 1\n    jp (hl)\n", 0, reg(Reg::A)),
        Usage::Unknown
    );
}

/// Reaching data as if it were code means the analysis lost the thread.
#[test]
fn falling_into_data_is_unknown() {
    assert_eq!(
        usage("    ld a, 1\n    defb 0, 1, 2\n", 0, reg(Reg::A)),
        Usage::Unknown
    );
}

/// Flags work exactly like registers, including being killed by an
/// instruction that rewrites them.
#[test]
fn a_flag_overwritten_before_any_test_is_not_used() {
    assert_eq!(
        usage(
            "    or a\n    ld a, 5\n    cp 3\n    jr z, done\ndone:\n    ret\n",
            0,
            Dependency::Flag(Flag::Z)
        ),
        Usage::NotUsed
    );
}

#[test]
fn a_flag_tested_by_a_later_branch_is_used() {
    assert_eq!(
        usage(
            "    or a\n    ld a, 5\n    jr z, done\ndone:\n    ret\n",
            0,
            Dependency::Flag(Flag::Z)
        ),
        Usage::Used
    );
}
