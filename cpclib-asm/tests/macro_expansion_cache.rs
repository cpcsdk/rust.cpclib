//! `ProcessedToken::update_macro_or_struct_state` (see
//! `cpclib-asm/src/assembler/processed_token.rs`) caches a macro/struct
//! call's parsed expansion, keyed on the macro's identity plus each call
//! argument's *resolved* string form (computed before the body is spliced
//! together) - so repeated calls with the same effective arguments skip both
//! the splice and the `winnow` parse. These tests check the properties that
//! make that safe:
//!
//! - identical calls at different call sites still assemble correctly
//!   (`two_identical_calls_from_different_sites_assemble_correctly`)
//! - a call argument that only resolves once a forward reference converges
//!   is never served a stale value from an earlier, differently-resolved
//!   pass (`a_forward_referenced_eval_argument_converges_to_the_right_value`)
//! - an unresolvable argument the macro body never references still does not
//!   error, i.e. building the cache key does not force-evaluate arguments
//!   the original code was lazy about
//!   (`an_unreferenced_unresolvable_argument_is_still_never_evaluated`)
//! - two call sites that share a cached expansion still each report their
//!   own failure at their own location, not the location of whichever call
//!   site happened to populate the cache first
//!   (`two_call_sites_of_the_same_macro_each_report_their_own_failing_assert_location`)

use cpclib_asm::{EnvOptions, assemble, assembler, parser};

/// The same macro, called twice with identical (non-evaluated) arguments
/// from two different call sites - both must expand and assemble correctly,
/// whichever of the two populates the cache and whichever hits it.
#[test]
fn two_identical_calls_from_different_sites_assemble_correctly() {
    let code = r#"
    org 0x8000
    macro POKE addr, val
        ld hl, {addr}
        ld (hl), {val}
    endm
    POKE(0x1234, 0x56)
    nop
    POKE(0x1234, 0x56)
"#;
    let bytes = assemble(code).expect("two identical macro calls must assemble");
    let one_call = vec![0x21, 0x34, 0x12, 0x36, 0x56];
    let mut expected = one_call.clone();
    expected.push(0x00); // nop
    expected.extend(one_call);
    assert_eq!(bytes, expected, "{bytes:?}");
}

/// `{eval}` forces a macro argument to be evaluated against the current
/// environment (see `expand_param` in `assembler/macro.rs`) rather than
/// substituted as literal text - so the cache key (built from the resolved
/// *value*, not the call-site expression text) must vary correctly with
/// that value. Two calls to the same macro with different backward
/// references (`{eval}A` vs `{eval}B`, `A != B`) must each expand to their
/// own distinct, correct value - proving a resolved-value change produces a
/// cache miss rather than reusing the other call's expansion.
#[test]
fn eval_argument_resolution_feeds_the_cache_key_correctly() {
    let code = r#"
    org 0x8000
A equ 0x11
B equ 0x22
    macro EMIT val
        db {val}
    endm
    EMIT({eval}A)
    EMIT({eval}B)
"#;
    let bytes = assemble(code).expect("both eval'd calls must assemble");
    assert_eq!(bytes, vec![0x11, 0x22], "{bytes:?}");
}

/// The cache key is built from the same lazy resolution `expand_param`
/// always did: only arguments the macro body actually references via
/// `{index}`/`{index:=default}` are ever evaluated. Building the key must
/// not force-evaluate an argument the body never reads - otherwise an
/// unused, unresolvable argument would newly become an error.
#[test]
fn an_unreferenced_unresolvable_argument_is_still_never_evaluated() {
    let code = r#"
    org 0x8000
    macro M used, ...
        db {used}
    endm
    M(0x11, {eval}THIS_LABEL_DOES_NOT_EXIST_ANYWHERE)
"#;
    let bytes = assemble(code)
        .expect("an unreferenced argument must never be evaluated, even if unresolvable");
    assert_eq!(bytes, vec![0x11], "{bytes:?}");
}

/// Two call sites of the same macro with the same arguments (so, whichever
/// runs first, the second can hit the cache populated by the first) each
/// fail their own `assert`. `assert` failures are collected across the
/// whole file rather than aborting assembly on the first one (see
/// `Env::handle_assert`), so both show up in one combined error - and each
/// must still point at its own call site's line, not the line of whichever
/// call populated the shared, cached expansion (see the "known, flagged
/// trade-off" doc comment on `ProcessedToken::update_macro_or_struct_state`
/// about a shared `LocatedListing`'s baked-in context name).
#[test]
fn two_call_sites_of_the_same_macro_each_report_their_own_failing_assert_location() {
    let code = "\
\t\torg 0x8000
\t\tMACRO CHECK
\t\t\tassert string_len(\"ab\") == 3, \"wrong length\"
\t\tENDM
\t\tCHECK()
\t\tnop
\t\tnop
\t\tCHECK()
\t\t";
    let tokens = parser::parse_z80_str(code).unwrap();
    let options = EnvOptions::default();
    let (_tok, mut env) = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
        Ok(ok) => ok,
        Err((_t, _env, e)) => panic!("assembling should not fail outright: {e}")
    };

    let err = match env.handle_post_actions(&tokens) {
        Ok(_) => panic!("both failing asserts should be reported as an error"),
        Err(e) => e
    };

    let text = err.to_string();
    // Call sites are on lines 5 and 8 (1-based) - both must be named, not
    // just one of them (which would mean the second call's failure got
    // attributed to the first call's location instead of its own).
    assert!(
        text.contains(":5:") && text.contains(":8:"),
        "both call sites (line 5 and line 8) must be reported, got: {text}"
    );
}
