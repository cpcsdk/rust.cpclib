//! `UNION ... NEXTU ... ENDU` - several alternative "member" listings share
//! the same starting address, like a C union. Every member always executes
//! (unlike `IF`/`SWITCH`'s conditional branches); `$` after `ENDU` advances
//! by the MAX size any member reached, not their sum. Member bodies are
//! grammar-restricted to data directives and labels (`ParsingState::
//! UnionLimited`), broadened from rgbds' own `DS`-only precedent to also
//! allow real data (`DB`/`DW`/`STR`/a struct instantiation).

/// `cpclib_asm::assemble`/`assemble_with_options` never call
/// `Env::handle_post_actions` (only the CLI does, via a separate explicit
/// call) - an inline `assert` in the source never fails the returned
/// `Result` through that path, it is only ever collected and would be
/// printed later by something that calls `handle_post_actions` itself. A
/// test that only checks `.is_ok()` on `cpclib_asm::assemble(...)`'s result
/// for code containing `assert` statements is not actually checking those
/// asserts passed - confirmed live (`assert 1 == 2\n` alone still returns
/// `Ok(())` through the bare `assemble` path). This helper closes that gap
/// by driving the same two-step pipeline the CLI uses, so a wrong assert
/// here is a real test failure, not silently ignored.
fn assemble_checking_asserts(code: &str) -> Result<Vec<u8>, Box<cpclib_asm::error::AssemblerError>> {
    let tokens = cpclib_asm::parser::parse_z80_str(code)?;
    let options = cpclib_asm::EnvOptions::default();
    let (_tok, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(&tokens, options)
        .map_err(|(_, _, e)| e)?;
    env.handle_post_actions(&tokens)?;
    Ok(env.produced_bytes())
}

#[test]
fn two_members_share_one_address() {
    let bin = assemble_checking_asserts(
        "org 0x4000\n UNION\n db 1\n NEXTU\n dw 2\n ENDU\n assert $ == 0x4002\n db 0xAA\n"
    )
    .unwrap();
    // the assembled content at the overlapping address is always whichever
    // member came last (`dw 2`'s bytes), not `db 1`'s.
    assert_eq!(bin, vec![0x02, 0x00, 0xAA]);
}

#[test]
fn size_is_the_true_max_across_three_members() {
    let bin = assemble_checking_asserts(
        "org 0x4000\n UNION\n db 1\n NEXTU\n ds 5\n NEXTU\n dw 2\n ENDU\n assert $ == 0x4005\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn labels_in_different_members_alias_the_same_address() {
    // rgbds' own canonical example, translated - basm's own single-colon
    // label syntax, not rgbds' `::` export marker (basm doesn't parse `::`
    // as part of a label definition here - `parse_label`'s own
    // `doubledots` parameter is `false` at this call site).
    let bin = assemble_checking_asserts(
        "org 0x4000\n UNION\n wName: ds 10\n NEXTU\n wHealth: ds 2\n wVideoBuffer: ds 16\n ENDU\n \
         assert wName == 0x4000\n assert wHealth == 0x4000\n assert wVideoBuffer == 0x4002\n \
         assert $ == 0x4012\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn ds_only_member_the_strict_rgbds_shape_works() {
    let bin = assemble_checking_asserts(
        "org 0x4000\n UNION\n ds 4\n NEXTU\n ds 8\n ENDU\n assert $ == 0x4008\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn struct_instantiation_member_the_broadened_shape_works() {
    // `p.x`/`p.y` now resolve for real (a genuine fix, not a workaround -
    // struct instantiation used to never emit per-field instance labels at
    // all, so `instance.field` access was silently broken everywhere, not
    // just here - see `StructWithArgs::expand` in
    // `cpclib-asm/src/assembler/macro.rs`).
    let bin = assemble_checking_asserts(
        "STRUCT Point\n x db 0\n y db 0\n ENDSTRUCT\n org 0x4000\n UNION\n ds 4\n NEXTU\n p: \
         Point(void)\n ENDU\n assert p.x == 0x4000\n assert p.y == 0x4001\n assert $ == 0x4004\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn single_member_no_nextu_is_legal() {
    let bin = assemble_checking_asserts("org 0x4000\n UNION\n db 1, 2\n ENDU\n assert $ == 0x4002\n");
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn an_empty_member_is_legal() {
    let bin = assemble_checking_asserts(
        "org 0x4000\n UNION\n NEXTU\n db 1\n ENDU\n assert $ == 0x4001\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn a_union_can_nest_inside_a_union_member() {
    let bin = assemble_checking_asserts(
        "org 0x4000\n UNION\n ds 1\n NEXTU\n UNION\n db 1\n NEXTU\n dw 2\n ENDU\n ENDU\n assert \
         $ == 0x4002\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn overlap_outside_a_union_is_still_flagged_under_forbid_memory_override() {
    let mut assemble_opts = cpclib_asm::AssemblingOptions::default();
    assemble_opts.set_forbid_memory_override(true);
    let opts = cpclib_asm::EnvOptions::from(assemble_opts);

    // a genuine, real accidental overlap OUTSIDE any union must still be a
    // hard error - the union's own suppression must not leak past it.
    let err = cpclib_asm::assemble_with_options(
        "org 0x4000\n db 1, 2, 3\n org 0x4000\n db 4\n",
        opts
    );
    assert!(err.is_err(), "{err:?}");
}

#[test]
fn union_overlap_is_silent_even_under_forbid_memory_override() {
    let mut assemble_opts = cpclib_asm::AssemblingOptions::default();
    assemble_opts.set_forbid_memory_override(true);
    let opts = cpclib_asm::EnvOptions::from(assemble_opts);

    // the whole point of UNION: intentional overlap must not error even
    // when forbid_memory_override is on.
    let res = cpclib_asm::assemble_with_options(
        "org 0x4000\n UNION\n db 1, 2, 3\n NEXTU\n dw 0xAABB\n ENDU\n",
        opts
    );
    assert!(res.is_ok(), "{res:?}");
}

#[test]
fn a_real_instruction_inside_a_member_is_rejected() {
    let err = cpclib_asm::assemble("org 0x4000\n UNION\n ld a, 1\n ENDU\n");
    assert!(err.is_err(), "{err:?}");
}

#[test]
fn union_itself_is_rejected_inside_a_function_body() {
    // A FUNCTION body computes a value via RETURN, never emits bytes/
    // addresses - UNION's entire purpose is byte/address layout, which has
    // no meaning there, unlike SWITCH (pure control flow) which is fine.
    let err = cpclib_asm::assemble(
        "FUNCTION f\nUNION\ndb 1\nENDU\nRETURN 1\nENDFUNCTION\norg 0x4000\ndb f()\n"
    );
    assert!(err.is_err(), "{err:?}");
}

#[test]
fn union_itself_is_rejected_inside_a_struct_body() {
    let err = cpclib_asm::assemble("STRUCT S\nUNION\ndb 1\nENDU\nENDSTRUCT\n");
    assert!(err.is_err(), "{err:?}");
}

#[test]
fn unterminated_union_is_a_clear_parse_error() {
    // Trailing `ret` after the unclosed UNION, not left to hit true EOF
    // mid-block: that shape hits a separate, pre-existing winnow `repeat`
    // panic-safety bug shared with SWITCH (confirmed live - "org 0x4000\n
    // SWITCH 1\n CASE 1\n db 1\n" with no ENDSWITCH panics identically,
    // nothing to do with UNION specifically) rather than a clean parse
    // error - out of scope here, a real file always has more content after
    // a forgotten ENDU anyway.
    let err = cpclib_asm::assemble("org 0x4000\n UNION\n db 1\n ret\n");
    assert!(err.is_err(), "{err:?}");
}

#[test]
fn a_for_loop_still_works_inside_a_function_body() {
    // Regression lock for a real, separate pre-existing gap found and fixed
    // while implementing UNION's own FunctionLimited/StructLimited
    // exclusion: block directives parsed via parse_z80_directive_with_block
    // (SWITCH, UNION, FOR, WHILE, MODULE, ...) never actually went through
    // the ParsingState::is_accepted check at all - so a FOR loop inside a
    // FUNCTION body (used for real in cpclib-basm/tests/asm/
    // good_function_load.asm's REVERT function) was only ever working by
    // accident, not because it was deliberately allowed. Fixing the general
    // gap (needed for UNION's own exclusion to mean anything) meant FOR
    // (and, consistently, WHILE) had to be added to FunctionLimited's own
    // allow-list explicitly, rather than staying accidentally permitted.
    let bin = assemble_checking_asserts(
        "FUNCTION revert, l\nnew = []\nnb = list_len({l})\nfor idx, 0, nb-1\nnew = \
         list_push(new, list_get({l}, nb-1-{idx}))\nendfor\nreturn new\nENDFUNCTION\norg \
         0x4000\nassert revert([1, 2, 3]) == [3, 2, 1]\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}
