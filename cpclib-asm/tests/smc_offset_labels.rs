//! sjasmplus-style "SMC offset" labels: `label+N:` (manual) and `label+*:`
//! ("smart", inferred from the following instruction) - for self-modifying-
//! code runtime patching.

#[test]
fn manual_offset_resolves_relative_to_the_label() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n foo: ld a,13\n answer+1: ld a,13\n db peek(answer)\n"
    )
    .unwrap();
    assert_eq!(bin, vec![0x3E, 13, 0x3E, 13, 13]);
}

#[test]
fn manual_offset_address_matches_hand_computed_expectation() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n foo: ld a,13\n answer+1: ld a,13\n assert answer == foo + 3\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn manual_offset_not_followed_by_anything_still_assembles() {
    // Unlike the smart form, a manual offset doesn't need to derive
    // anything from what follows.
    let bin = cpclib_asm::assemble("org 0x4000\n answer+1:\n");
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_matches_manual_offset_on_an_identical_instruction() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n smart_start: answer+*: ld a,13\n manual_start: manual+1: ld a,13\n \
         assert answer - smart_start == manual - manual_start\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_ld_a_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: ld a, 13\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_jp_cc_nnnn() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: jp nz, 0x1234\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_ld_ix_nnnn() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: ld ix, 0x1234\n assert a == s + 2\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_ld_mem_nnnn_a() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: ld (0x1234), a\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_jr() {
    let bin = cpclib_asm::assemble("org 0x4000\n s: a+*: jr s\n assert a == s + 1\n");
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_djnz() {
    let bin = cpclib_asm::assemble("org 0x4000\n s: a+*: djnz s\n assert a == s + 1\n");
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_in_a_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: in a, (0x12)\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_out_n_a() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: out (0x12), a\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_ld_indexed_with_immediate_is_not_mistaken_for_the_ddcb_exception() {
    // `LD (IX+3),n` - the value byte (not the delta) is the tail, offset 3
    // (prefix, 0x36, delta, value) - must not be confused with the DDCB
    // bit-op exception below, which fixes the offset at 2 regardless.
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: ld (ix+3), 0x42\n assert a == s + 3\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_ddcb_exception_rlc() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: rlc (ix+2)\n assert a == s + 2\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_ddcb_exception_bit() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: bit 3, (iy+1)\n assert a == s + 2\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_and_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: and 0x12\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_add_a_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: add a, 0x12\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_adc_a_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: adc a, 0x12\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_sbc_a_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: sbc a, 0x12\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_sub_n() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*: sub 0x12\n assert a == s + 1\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_still_rejects_the_fake_16bit_add_form() {
    // `ADD DE,BC` is a fake pseudo-instruction that expands into several
    // real instructions - the tail-offset model doesn't apply, unlike the
    // real, single-instruction `ADD A,n` form covered above.
    let err = cpclib_asm::assemble("org 0x4000\n a+*: add de, bc\n").unwrap_err();
    assert!(format!("{err}").contains("smart SMC offset"), "{err}");
}

#[test]
fn smart_offset_survives_a_comment_between_label_and_instruction() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s: a+*:\n ; a comment\n ld a, 13\n assert a == s + 1\n ret\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_resolves_correctly_across_a_forward_reference_pass() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n ld hl, forward\n s: forward+*: ld a, 13\n assert forward == s + 1\n \
         ret\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn smart_offset_unsupported_mnemonic_is_an_assembling_error() {
    let err = cpclib_asm::assemble("org 0x4000\n a+*: nop\n").unwrap_err();
    assert!(format!("{err}").contains("smart SMC offset"));
}

#[test]
fn smart_offset_with_no_following_instruction_is_an_assembling_error() {
    let err = cpclib_asm::assemble("org 0x4000\n a+*:\n").unwrap_err();
    assert!(format!("{err}").contains("no following instruction"));
}

#[test]
fn smc_offset_combined_with_equ_is_a_parse_error() {
    assert!(cpclib_asm::assemble("org 0x4000\n a+1 EQU 5\n").is_err());
}

#[test]
fn smc_offset_before_a_bare_macro_call_is_a_parse_error() {
    let code = "org 0x4000\n MACRO foo\n nop\n ENDM\n a+1 foo()\n";
    assert!(cpclib_asm::assemble(code).is_err());
}

#[test]
fn smart_offset_colon_form_before_a_macro_call_resolves_through_the_expansion() {
    // With a colon, `a+*: foo()` is unambiguously a label followed by an
    // instruction (the macro call) - not the bare-form ambiguity above.
    // The pending marker survives macro expansion and resolves against the
    // macro's own first expanded instruction.
    let code = "org 0x4000\n MACRO foo\n ld a, 13\n ENDM\n s: a+*: foo()\n assert a == s + 1\n";
    let bin = cpclib_asm::assemble(code);
    assert!(bin.is_ok(), "{bin:?}");
}
