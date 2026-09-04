//! Regression test for a fixed soundness bug: `ProcessedToken::visited` used
//! to `transmute::<&T, &LocatedToken>` unconditionally, including for
//! `T = Token` - the type a user-defined `FUNCTION`'s body is evaluated
//! through. That path is only exercised while listing/source-map recording
//! is active (`output_trigger.is_some()`), so a plain `assemble()` call
//! doesn't reach it - this test explicitly enables source-map recording
//! (which turns on the same `output_trigger` machinery a `-l` listing does)
//! around a function call, the way the buggy code path required.

use std::sync::Arc;

use cpclib_common::event::DiscardObserver;

#[test]
fn calling_a_user_defined_function_while_listing_is_recorded_does_not_crash() {
    let source = r#"
        FUNCTION double, x
            return {x} * 2
        ENDFUNCTION

        ld a, double(21)
        assert double(21) == 42
    "#;

    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.record_source_map();

    let (_p, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(parse, assemble, Arc::new(DiscardObserver))
    )
    .expect("assembles without crashing");
    env.handle_post_actions(&listing)
        .expect("post actions succeed");
}
