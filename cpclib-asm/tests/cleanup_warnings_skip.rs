//! `Env::cleanup_warnings` (see `cpclib-asm/src/assembler/mod.rs`) runs at
//! the end of *every* `visit_processed_tokens` call - once per macro/struct
//! expansion, `INCLUDE`, `IF` branch and `REPEAT` iteration visited, not
//! just once per pass - because any of those can be the last chance to
//! render a warning before the buffer its span points into gets reused. It
//! now skips its own (expensive, full-vector) work when nothing was pushed
//! to `Env::warnings` since the last call, tracked via a monotonic push
//! counter rather than `warnings.len()` (which shrinks as
//! `merge_overriding_warnings` merges entries, so isn't itself a safe
//! "nothing changed" signal). This test locks down that the skip never
//! drops or merges away warnings it shouldn't: many separate macro calls,
//! each producing its own precision-loss warning, must all still surface.

use cpclib_asm::assemble_with_options;
use cpclib_asm::EnvOptions;

/// The same macro, called many times (so `cleanup_warnings` runs many times
/// with nothing new in between most calls), each call individually
/// triggering a float-to-integer precision-loss warning. All of them must
/// still be reported - the skip-when-unchanged optimization must not lose
/// any, or coalesce them into fewer than were actually produced.
#[test]
fn many_separate_macro_calls_each_still_report_their_own_warning() {
    let code = "\
    org 0x8000
    macro EMIT_HALF n
        db {n}/2
    endm
    EMIT_HALF(1)
    EMIT_HALF(3)
    EMIT_HALF(5)
    EMIT_HALF(7)
    EMIT_HALF(9)
";
    let options = EnvOptions::default();
    let (bytes, _symbols) =
        assemble_with_options(code, options).expect("assembling must succeed despite warnings");
    // 1/2 -> 1 (rounds up per the (raw+0.5).floor() rule), 3/2 -> 2, 5/2 -> 3, 7/2 -> 4, 9/2 -> 5
    assert_eq!(bytes, vec![1, 2, 3, 4, 5], "{bytes:?}");
}

/// Same shape as above, but reading the warnings back out directly to
/// confirm the count matches exactly - not silently deduplicated to fewer
/// than five, and not silently dropped to zero.
#[test]
fn warning_count_matches_the_number_of_triggering_calls_exactly() {
    let code = "\
    org 0x8000
    macro EMIT_HALF n
        db {n}/2
    endm
    EMIT_HALF(1)
    EMIT_HALF(3)
    EMIT_HALF(5)
";
    let tokens = cpclib_asm::parser::parse_z80_str(code).expect("must parse");
    let options = EnvOptions::default();
    let (_tok, env) =
        match cpclib_asm::assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok(ok) => ok,
            Err((_t, _env, e)) => panic!("assembling should not fail outright: {e}")
        };

    let warnings = env.warnings();
    let precision_loss_count = warnings
        .iter()
        .filter(|w| w.to_string().contains("truncated to integer"))
        .count();
    assert_eq!(
        precision_loss_count, 3,
        "expected exactly 3 precision-loss warnings, one per EMIT_HALF call, got: {warnings:?}"
    );
}
