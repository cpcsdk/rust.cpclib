//! Regression test for a real correctness bug: `RETURN` inside a nested block
//! (IF/REPEAT/SWITCH/...) within a `FUNCTION` did not stop the *rest of that
//! same block* from still executing.
//!
//! `RETURN` is only syntactically legal under `ParsingState::FunctionLimited`
//! (see `parser/context.rs`'s `is_accepted`), but that same state also allows
//! `IF`/`REPEAT`/`SWITCH`/etc. bodies, and those bodies are parsed via plain
//! `inner_code`, which propagates whatever parsing state is currently
//! ambient - so a nested `IF` inside a `FUNCTION` is parsed under
//! `FunctionLimited` too, and `RETURN` is legal inside it.
//!
//! At runtime, `ProcessedTokenState::If` executes its chosen branch via a
//! recursive call to `visit_processed_tokens` - the exact same shared loop
//! every nested construct funnels through. That loop only checked
//! `env.return_value.is_some()` once, at its own entry, never between the
//! tokens of its own per-token loop - unlike `AnyFunction::eval`'s hand-rolled
//! top-level loop over a FUNCTION's *direct* tokens, which does check after
//! every token. So a `RETURN` firing inside an `IF`'s body left every
//! following token in that *same* `IF` branch still executing.
//!
//! `ASSERT 0` right after `RETURN`, inside the `IF`, makes this a clean
//! pass/fail - but assert failures are *delayed* (queued as a
//! `FailedAssertCommand`, only turned into an `Err` by `handle_assert`/
//! `handle_post_actions`), so this goes through the same lower-level API
//! `function_listing_regression.rs` uses rather than the plain `assemble()`
//! wrapper, which never calls `handle_post_actions` and so would report
//! `Ok` either way.

use std::sync::Arc;

use cpclib_common::event::DiscardObserver;

#[test]
fn return_inside_a_nested_if_stops_the_rest_of_that_if_branch() {
    let source = r#"
        FUNCTION f
            IF 1
                RETURN 1
                ASSERT 0
            ENDIF
            RETURN 0
        ENDFUNCTION

        org 0x4000
        db f()
    "#;

    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let assemble = cpclib_asm::AssemblingOptions::default();

    let (_p, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(parse, assemble, Arc::new(DiscardObserver))
    )
    .expect("assembling itself must not fail");

    env.handle_post_actions(&listing).expect(
        "RETURN must stop the IF branch before the ASSERT is ever reached - a failure here means \
         the ASSERT still ran"
    );

    assert_eq!(
        env.produced_bytes(),
        vec![1],
        "f() must evaluate to 1, the IF branch's RETURN"
    );
}
