//! `{N:=default}` in a variadic macro body.
//!
//! A variadic macro's body legitimately references arguments that only *some*
//! calls pass, and the reference is usually inside a branch those calls never
//! take. Macro expansion is textual and happens before the Z80 parser runs, so
//! it cannot know which branch will be taken - it sees `{3}`, finds no third
//! argument, and fails, for a call whose taken branch never mentions one.
//!
//! `{2:=0}` is what the macro author writes to say "and if there is no such
//! argument, put this instead".
//!
//! Note the indexing: `{N}` is **0-based** over *all* arguments, named ones
//! included. For `macro M timeout, kind, ...` the first variadic extra is
//! `{2}`, not `{3}`.

/// The real macro this was built for, reduced to what matters.
///
/// Reported as: `argument {N} is referenced in the body of macro
/// REGISTER_EVENT, but only 2 argument(s) were provided at this call` - for a
/// call whose `switch` branch does not use the optional argument at all.
const EVENTS: &str = r#"
    org 0x4000
EVENT_CHANGE_PALETTE equ 1
EVENT_START_MUSIC    equ 2

    macro REGISTER_EVENT timeout, kind, ...
        dw {timeout}
        switch {kind}
            case EVENT_CHANGE_PALETTE
                assert {#} == 3
                dw {2:=0}
                break
            case EVENT_START_MUSIC
                assert {#} == 2
                break
            default
                fail "Unknown event kind: {kind}"
        endswitch
    endm
"#;

/// The call that used to fail: two arguments, and the branch it takes never
/// references a third.
#[test]
fn a_branch_that_needs_no_extra_argument_assembles() {
    let code = format!("{EVENTS}\n    REGISTER_EVENT(100, EVENT_START_MUSIC)\n");
    let bytes = cpclib_asm::assemble(&code)
        .unwrap_or_else(|e| panic!("a call not using the optional argument must assemble: {e}"));
    // `dw 100` only - the taken branch emits nothing else.
    assert_eq!(bytes, vec![100, 0], "{bytes:?}");
}

/// The other branch still receives the real argument when it is supplied - the
/// default must not shadow a value that exists.
#[test]
fn the_supplied_argument_still_wins_over_the_default() {
    let code = format!("{EVENTS}\n    REGISTER_EVENT(100, EVENT_CHANGE_PALETTE, 0x1234)\n");
    let bytes = cpclib_asm::assemble(&code).unwrap_or_else(|e| panic!("assemble failed: {e}"));
    assert_eq!(bytes, vec![100, 0, 0x34, 0x12], "{bytes:?}");
}

/// A default is arbitrary body text, not just a literal.
#[test]
fn a_default_may_be_an_expression() {
    let code = r#"
    org 0x4000
BASE equ 0x10
    macro M a, ...
        db {1:=BASE + 5}
    endm
    M(1)
"#;
    let bytes = cpclib_asm::assemble(code).expect("assemble failed");
    assert_eq!(bytes, vec![0x15]);
}

/// Without a default, the old error still stands - this is an opt-in, not a
/// silent "missing arguments are zero" rule that would hide real mistakes.
#[test]
fn a_reference_without_a_default_still_errors_when_unsupplied() {
    let code = r#"
    org 0x4000
    macro M a, ...
        db {1}
    endm
    M(1)
"#;
    let error = cpclib_asm::assemble(code)
        .expect_err("a missing argument with no default must still be an error");
    let text = error.to_string();
    assert!(
        text.contains("argument") && text.contains('1'),
        "the error must still name the missing argument: {text}"
    );
}
