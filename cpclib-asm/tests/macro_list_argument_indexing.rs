//! `{l}[{idx}]` in a macro body, where the call passes a list *literal*
//! for `l` (`SOME_MACRO([1, 2, 3], 0)`).
//!
//! Macro expansion is textual: `{l}` alone substitutes a list argument as a
//! bare, bracket-free comma list (`1,2,3`), by design - that is exactly what
//! makes `DB {l}` spread a list argument across a data line. But that same
//! flattened form breaks immediately if the macro body then indexes it with
//! `[...]`: `db 1,2,3[0]` parses as three separate `DB` arguments (`1`, `2`,
//! `3[0]` - "index into the literal `3`"), which is nonsense and errors
//! ("3 is not a list"), not what indexing into the list itself ever meant.
//!
//! Fixed by having `tokenize_macro_body` (`cpclib-tokens/src/macro_segment.rs`)
//! record whether a `{name}`/`{N}`/`{N:=default}` placeholder is immediately
//! followed by a literal `[` in the macro's own body text, and having
//! `finish_expand_for_basm`/`resolve_referenced_args`
//! (`cpclib-asm/src/assembler/macro.rs`) re-wrap the substitution in
//! `[...]` when that is true *and* the argument actually supplied at that
//! call is itself a list - so `[1, 2, 3][0]` is what actually gets spliced
//! in, which already parses correctly (bracket indexing on a list literal
//! is its own, separately-shipped feature). A single argument that merely
//! *evaluates* to a list (an identifier, a function call, ...) is
//! unaffected either way: it substitutes as plain text and indexes
//! correctly with no wrapping needed.

/// The bug as originally hit: a two-parameter macro indexing its list
/// parameter.
#[test]
fn a_list_literal_argument_can_be_indexed_in_the_macro_body() {
    let code = r#"
    MACRO GET(l, idx)
        db {l}[{idx}]
    ENDM
    org 0x4000
    GET([10, 20, 30], 1)
"#;
    let bytes = cpclib_asm::assemble(code)
        .unwrap_or_else(|e| panic!("indexing a list-literal macro argument must work: {e}"));
    assert_eq!(bytes, vec![20], "{bytes:?}");
}

/// The existing "spread a list across a DB line" idiom this fix must not
/// disturb - `{l}` alone (no following `[`) stays a flat, bracket-free
/// comma list.
#[test]
fn an_unindexed_list_argument_still_spreads_flat() {
    let code = r#"
    MACRO SPREAD(l)
        db {l}
    ENDM
    org 0x4000
    SPREAD([1, 2, 3])
"#;
    let bytes = cpclib_asm::assemble(code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![1, 2, 3], "{bytes:?}");
}

/// Both uses of the very same parameter, in the very same macro body: one
/// spread, one indexed. The wrapping decision is per *occurrence*
/// (`MacroSegment`), not per argument index, so this must not force a
/// single, wrong choice for both.
#[test]
fn the_same_list_argument_can_be_both_spread_and_indexed_in_one_body() {
    let code = r#"
    MACRO BOTH(l)
        db {l}
        db {l}[0]
    ENDM
    org 0x4000
    BOTH([9, 8, 7])
"#;
    let bytes = cpclib_asm::assemble(code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![9, 8, 7, 9], "{bytes:?}");
}

/// A single (non-list-literal) argument - even one that evaluates to a list
/// at runtime - substitutes as plain text and already indexes correctly on
/// its own; the fix must not wrap it too (that would turn `mylist[2]` into
/// the nonsensical `[mylist][2]`, a one-element list containing `mylist`).
#[test]
fn an_identifier_argument_that_merely_evaluates_to_a_list_needs_no_wrapping() {
    let code = r#"
    MACRO GET2(l, idx)
        db {l}[{idx}]
    ENDM
    mylist = [5, 6, 7]
    org 0x4000
    GET2(mylist, 2)
"#;
    let bytes = cpclib_asm::assemble(code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![7], "{bytes:?}");
}

/// Variadic positional references (`{0}`/`{1}`, not named params) get the
/// same treatment.
#[test]
fn a_variadic_positional_list_argument_can_be_indexed() {
    let code = r#"
    MACRO V(...)
        db {0}[{1}]
    ENDM
    org 0x4000
    V([11, 22, 33], 2)
"#;
    let bytes = cpclib_asm::assemble(code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![33], "{bytes:?}");
}

/// `{N:=default}` (`ArgOr`) gets the same treatment when the call actually
/// supplies a list argument for it.
#[test]
fn a_default_carrying_reference_can_be_indexed_when_a_list_is_supplied() {
    let code = r#"
    MACRO W(...)
        db {0:=[10, 20]}[1]
    ENDM
    org 0x4000
    W([100, 200, 201])
"#;
    let bytes = cpclib_asm::assemble(code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![200], "{bytes:?}");
}

/// The default text itself is untouched (already whatever the macro author
/// wrote, brackets included) when no argument is supplied at all - no
/// wrapping is applied to it, since it is emitted verbatim, never
/// `is_list()`-checked (there is no supplied argument to check).
#[test]
fn a_default_carrying_reference_indexes_its_own_default_when_unsupplied() {
    let code = r#"
    MACRO W(...)
        db {0:=[10, 20]}[1]
    ENDM
    org 0x4000
    W()
"#;
    let bytes = cpclib_asm::assemble(code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![20], "{bytes:?}");
}
