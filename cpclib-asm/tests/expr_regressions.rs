use cpclib_asm::preamble::*;

#[test]
fn test_negative_expression() {
    let exp = Expr::Value(-18);
    let val = exp.eval().unwrap();

    assert_eq!(val.int_value().unwrap(), -18);
}

/// `cpclib_asm::assemble` never calls `Env::handle_post_actions`, so a
/// failed `ASSERT` in the source never turns into an `Err` through that
/// path (confirmed in an earlier session: `assemble("assert 1 == 2\n")`
/// alone still returns `Ok(())`) - these tests drive the same two-step
/// pipeline the CLI uses instead, so a wrong assert here is a real test
/// failure, not silently ignored. See `cpclib-asm/tests/union.rs`'s own
/// copy of this same helper.
fn assemble_checking_asserts(code: &str) -> Result<Vec<u8>, Box<cpclib_asm::error::AssemblerError>> {
    let tokens = cpclib_asm::parser::parse_z80_str(code)?;
    let options = cpclib_asm::EnvOptions::default();
    let (_tok, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(&tokens, options)
        .map_err(|(_, _, e)| e)?;
    env.handle_post_actions(&tokens)?;
    Ok(env.produced_bytes())
}

/// A literal at i32::MIN's magnitude (`-2147483648`) used to crash the
/// whole assembler process: `negative_number`
/// (`cpclib-asm/src/parser/expression.rs`) parsed it as `-v` on a plain
/// `i32`, an unchecked negation that panics in a debug build for exactly
/// this one value (there is no positive `i32` equal to `2147483648`).
/// Wraps back to itself now, matching `ExprResult::Neg`'s own convention.
///
/// Deliberately checks `.is_ok()` only, never formats the error on failure:
/// `Box<AssemblerError>` returned by `assemble_checking_asserts` can carry
/// a span into the `LocatedListing` built inside that same call, and this
/// crate's own `project_asm_error_span_lifetime_footgun` note documents
/// that formatting it after that listing has already been dropped (as it
/// has been, once the function returns) is unsound - a known, deliberately
/// undocumented-in-code footgun of the low-level API, not something to
/// paper over in a one-off test by risking the same UB it warns about.
#[test]
fn negating_i32_min_wraps_instead_of_panicking() {
    let result = assemble_checking_asserts("org 0x4000\n assert -2147483648 == -2147483648\n");
    assert!(result.is_ok(), "assert should have held after wrapping negation");
}

/// Plain arithmetic overflow (no macros, no `{*}`, nothing exotic) also
/// used to panic the process - `ExprResult`'s `Add`/`Sub`/`Mul`/`Rem`
/// impls (`cpclib-tokens/src/tokens/expression.rs`) used the bare `i32`/
/// `u8` operators instead of their `wrapping_*` equivalents, unlike
/// `shr_checked`/`shl_checked` right next to them, which already used
/// `wrapping_shr`/`wrapping_shl`. See the previous test for why this only
/// checks `.is_ok()`, never formats the error.
#[test]
fn integer_overflow_wraps_instead_of_panicking() {
    let result = assemble_checking_asserts(
        "org 0x4000\n\
         assert 2147483647 + 1 == -2147483648\n\
         assert -2147483648 - 1 == 2147483647\n\
         assert 2147483647 * 2 == -2\n"
    );
    assert!(result.is_ok(), "wrapping-arithmetic asserts should have held");
}
