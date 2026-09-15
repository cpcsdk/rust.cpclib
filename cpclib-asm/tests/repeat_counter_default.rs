//! `REPEAT count, counter` (no explicit `start`) - the counter is **1-based**
//! by default, not 0-based.
//!
//! Confirmed as the real, intentional, load-bearing contract (not just
//! incidental current behavior) by three independent, pre-existing fixtures
//! that all depend on it: `good_repeat_incbin2.asm`/`good_repeat_include2.asm`
//! (`repeat 3, count` loading `..._1`/`..._2`/`..._3`-suffixed files - there is
//! no `..._0` file) and `good_macro_tokenization_check.asm` (`repeat 3,
//! channelNumber` reaching registers 8/9/10 via `{channelNumber} + 7`, i.e.
//! channelNumber = 1, 2, 3). `docs/basm/directives.md` used to claim the
//! opposite ("0-based by default") while embedding `good_repeat_incbin2.asm`
//! as its own example - directly contradicting itself. Fixed the doc instead
//! of the code (`Env::repeat_start` defaults to `1`), since changing the
//! default would break those three real fixtures. This test exists so the
//! contract has one clear, direct, explicit lock instead of only being
//! pinned indirectly through file-naming side effects.

/// The counter's very first value, with no explicit `start`, is 1 - not 0.
#[test]
fn a_single_iteration_repeat_with_no_explicit_start_uses_counter_value_one() {
    let bin = cpclib_asm::assemble("org 0x4000\n repeat 1, i\n db {i}\n rend\n")
        .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![1], "{bin:?}");
}

/// Three iterations count 1, 2, 3 - not 0, 1, 2.
#[test]
fn a_multi_iteration_repeat_with_no_explicit_start_counts_from_one() {
    let bin = cpclib_asm::assemble("org 0x4000\n repeat 3, i\n db {i}\n rend\n")
        .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![1, 2, 3], "{bin:?}");
}

/// An explicit `start` of 0 is exactly how to opt into 0-based counting -
/// `REPEAT count, counter, 0`.
#[test]
fn an_explicit_zero_start_makes_the_counter_zero_based() {
    let bin = cpclib_asm::assemble("org 0x4000\n repeat 3, i, 0\n db {i}\n rend\n")
        .unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bin, vec![0, 1, 2], "{bin:?}");
}
