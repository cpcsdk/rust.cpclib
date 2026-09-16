//! `{*}` and `{*[indexes]}` in a `MACRO` body.
//!
//! `{*}` expands *every* argument actually passed at a call, joined with
//! `,` - like `{list_arg}`'s own flat, bracket-free spread (`DB {list_arg}`),
//! but for the whole call rather than one list-valued argument.
//!
//! `{*[indexes]}` expands a *subset*, selected by `indexes` - a single
//! index, a list of indices (`[0, 2]`), or a range (`0..2`). `indexes` is
//! not fixed at macro-definition time: it is itself substituted (so it can
//! reference `{#}` or a named parameter) and then evaluated as a real
//! expression once a specific call is being expanded - `{*[{#}-1]}` always
//! selects the last argument actually passed, whatever that call's total
//! count turns out to be.
//!
//! Both are available unconditionally (not gated on the macro declaring a
//! trailing `...`) - `*` can never collide with a declared parameter name,
//! unlike the positional `{N}`/`{#}` forms.
//!
//! Both also support a further `[i]` right after them (`{*}[0]`,
//! `{*[0..2]}[1]`), indexing straight into the spread/selected result -
//! mirroring the bracket-rewrapping fix for a plain list-valued `{name}[i]`.
//! Unlike that case, wrapping here is unconditional (no "is the argument
//! actually a list" check needed): `{*}`/`{*[indexes]}` always produce a
//! list-shaped value, even a single selected argument.

#[test]
fn star_expands_every_argument_joined_with_commas() {
    let bin = cpclib_asm::assemble(
        "MACRO ALLARGS(...)\n db {*}\nENDM\n org 0x4000\n ALLARGS(1, 2, 3)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![1, 2, 3], "{bin:?}");
}

#[test]
fn star_works_for_named_parameters_too_not_just_variadic_extras() {
    // No trailing `...` at all - `{*}` is not gated behind variadic.
    let bin = cpclib_asm::assemble(
        "MACRO THREE(a, b, c)\n db {*}\nENDM\n org 0x4000\n THREE(4, 5, 6)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![4, 5, 6], "{bin:?}");
}

#[test]
fn star_with_a_range_selects_a_contiguous_sublist() {
    // `0..2` is exclusive (Rust range semantics) - indices 0 and 1 only.
    let bin = cpclib_asm::assemble(
        "MACRO SEL(...)\n db {*[0..2]}\nENDM\n org 0x4000\n SEL(10, 20, 30)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![10, 20], "{bin:?}");
}

#[test]
fn star_with_an_inclusive_range_includes_the_end() {
    let bin = cpclib_asm::assemble(
        "MACRO SEL(...)\n db {*[0..=2]}\nENDM\n org 0x4000\n SEL(10, 20, 30)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![10, 20, 30], "{bin:?}");
}

#[test]
fn star_with_a_literal_list_of_indices_selects_in_the_given_order() {
    // Also exercises a gather that is neither sorted nor contiguous, and
    // the nested-bracket tokenization (`[[0, 2]]`).
    let bin = cpclib_asm::assemble(
        "MACRO SEL(...)\n db {*[[2, 0]]}\nENDM\n org 0x4000\n SEL(10, 20, 30)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![30, 10], "{bin:?}");
}

#[test]
fn star_with_a_single_index_selects_just_that_one_argument() {
    let bin = cpclib_asm::assemble(
        "MACRO SEL(...)\n db {*[1]}\nENDM\n org 0x4000\n SEL(10, 20, 30)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![20], "{bin:?}");
}

#[test]
fn star_selector_can_reference_the_argument_count() {
    // `{#}` inside the selector - always the *last* argument, whatever a
    // given call's total count is.
    let bin = cpclib_asm::assemble(
        "MACRO LAST(...)\n db {*[{#}-1]}\nENDM\n org 0x4000\n LAST(10, 20, 30)\n LAST(40, 50)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![30, 50], "{bin:?}");
}

#[test]
fn star_selector_can_reference_a_named_parameter() {
    let bin = cpclib_asm::assemble(
        "MACRO SEL(idx, ...)\n db {*[{idx}]}\nENDM\n org 0x4000\n SEL(1, 10, 20, 30)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    // args are [idx=1, 10, 20, 30] - {*[{idx}]} selects args[1] = 10.
    assert_eq!(bin, vec![10], "{bin:?}");
}

#[test]
fn an_argument_referenced_both_directly_and_via_star_expands_consistently() {
    // Regression lock: {*}/{*[indexes]} used to call expand_param directly,
    // bypassing the outer body's own per-argument memoization
    // (expanded_args) that {index}/{name} segments share - so an argument
    // referenced both ways got evaluated twice. Both references must still
    // agree, and (per the fix) only cost one evaluation of the shared
    // {eval}-marked expression.
    let bin = cpclib_asm::assemble(
        "MACRO M(...)\n db {0}\n db {*}\nENDM\n org 0x4000\n M({eval}1+1)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![2, 2], "{bin:?}");
}

#[test]
fn a_named_parameter_used_only_inside_a_star_selector_is_not_flagged_unused() {
    // Regression lock: `unused_macro_parameter_indices` used to only look
    // at top-level `Arg` segments, so a parameter referenced exclusively
    // inside `{*[...]}`'s own (separately re-tokenized) text produced a
    // false "'idx' is never used" warning.
    let code = "MACRO SEL(idx, ...)\n db {*[{idx}]}\nENDM\n org 0x4000\n SEL(1, 42)\n";
    let tokens = cpclib_asm::parser::parse_z80_str(code).unwrap();
    let options = cpclib_asm::EnvOptions::default();
    let (_tok, env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(&tokens, options)
        .unwrap_or_else(|(_, _, e)| panic!("assembling should not fail outright: {e}"));
    let warnings: Vec<String> = env.warnings().iter().map(|w| w.to_string()).collect();
    assert!(
        !warnings.iter().any(|w| w.contains("never used")),
        "{warnings:?}"
    );
}

#[test]
fn star_selector_out_of_range_index_is_a_clear_error() {
    let err = cpclib_asm::assemble(
        "MACRO SEL(...)\n db {*[5]}\nENDM\n org 0x4000\n SEL(10, 20)\n"
    );
    let err = err.expect_err("an out-of-range {*[...]} index must be a clear error");
    let text = err.to_string();
    assert!(text.contains("out of range"), "{text}");
}

#[test]
fn star_selector_negative_index_is_a_clear_error() {
    let err = cpclib_asm::assemble(
        "MACRO SEL(...)\n db {*[0-1]}\nENDM\n org 0x4000\n SEL(10, 20)\n"
    );
    assert!(err.is_err(), "{err:?}");
}

#[test]
fn star_alone_still_works_with_only_named_parameters_and_no_variadic_extras() {
    let bin = cpclib_asm::assemble(
        "MACRO PAIR(a, b)\n dw {*}\nENDM\n org 0x4000\n PAIR(0x1234, 0x5678)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![0x34, 0x12, 0x78, 0x56], "{bin:?}");
}

#[test]
fn star_alone_can_be_indexed_right_after_it() {
    let bin = cpclib_asm::assemble(
        "MACRO FIRST(...)\n db {*}[0]\nENDM\n org 0x4000\n FIRST(11, 22, 33)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![11], "{bin:?}");
}

#[test]
fn star_selector_result_can_be_indexed_right_after_it() {
    let bin = cpclib_asm::assemble(
        "MACRO SEL_FIRST(...)\n db {*[0..2]}[1]\nENDM\n org 0x4000\n SEL_FIRST(11, 22, 33)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    // {*[0..2]} selects [11, 22]; [1] then picks the second one, 22.
    assert_eq!(bin, vec![22], "{bin:?}");
}

#[test]
fn star_alone_without_a_following_bracket_still_spreads_flat() {
    // Regression guard for the indexing fix above: `{*}` with no `[...]`
    // right after it must keep spreading flat, not get wrapped anyway.
    let bin = cpclib_asm::assemble(
        "MACRO ALL(...)\n db {*}\nENDM\n org 0x4000\n ALL(1, 2, 3)\n"
    )
    .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![1, 2, 3], "{bin:?}");
}
