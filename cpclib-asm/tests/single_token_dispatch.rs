//! `parse_single_token` (see `cpclib-asm/src/parser/common.rs`) tries the
//! mnemonic dispatch table before the directive dispatch table for a line's
//! leading word, sharing one scan of that word between both instead of each
//! rescanning it independently. These tests lock down the edge cases that
//! made that fusion easy to get subtly wrong:
//!
//! - the `SL1` and `SRL8` opcode aliases, whose recognition depends on where
//!   exactly the leading-word scan stops relative to the digit glued onto
//!   `SL`/`SRL` - `SRL8` specifically was missed in the first pass of this
//!   fix (caught by `cpclib-basm`'s own test suite, `good_fake_instructions4.asm`)
//!   since it's dispatched through a nested `alt()` branch (`parse_srl8`)
//!   rather than an inline digit literal at the match-arm call site, so a
//!   textual grep for the `SL`/`SL1` pattern didn't find it. Cross-checked
//!   against every digit-suffixed `Mnemonic` variant in the enum (`Sl1`,
//!   `Srl8`, `Nop2`) - `Nop2`/`NOPS2` is confirmed separately as pre-existing
//!   dead code, unreachable before this work too (its table's own scan is
//!   untouched by this change), so it's intentionally not covered here.
//! - `NOP` (and other directive-table-only mnemonics), which must still
//!   dispatch correctly now that both tables are tried against a single
//!   shared scan of the word
//! - a label whose name happens to share a directive/mnemonic's shape plus a
//!   trailing digit (`DB2`), which must still fall through to label parsing
//!   rather than being mistaken for a dispatch-table entry
//! - a hard parse failure inside a recognized mnemonic's operand (`cut_err`,
//!   e.g. `PUSH` with an invalid register) must still propagate immediately
//!   with its precise location, not be silently discarded in favor of
//!   trying the directive table next

use std::ops::Deref;

use cpclib_asm::assemble;
use cpclib_asm::parser::{ParserContext, parse_single_token, parse_token};
use cpclib_asm::preamble::*;

fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
    let mut ctx = Box::new(ParserContextBuilder::default().build(code));
    ctx.context_name = Some("TEST".into());
    let span = Z80Span::new_extra(code, ctx.deref());
    (ctx, span)
}

/// `SL1` is an alias for `SLL`. A bare `SL1 B` statement is intercepted
/// earlier, at the statement level (label/macro-call disambiguation), before
/// `parse_single_token`'s dispatch tables are even tried - confirmed
/// pre-existing and unrelated to this change (`assemble("SL1 B\n")` fails
/// identically on unmodified `common.rs`/`directives.rs`), so these tests
/// exercise the dispatch functions directly instead, the way
/// `tests/parse_regression.rs`'s own `parse_token(&mut span.into())` tests
/// do.
///
/// `parse_token` (mnemonic-only, `parse_token2`'s own standalone,
/// alpha-only scan) must still recognize `SL1` the original way: `word ==
/// "SL"`, then a hand-consumed literal `'1'` (the `"SL"` arm in
/// `dispatch_token2`, `common.rs`) - unchanged by this work.
#[test]
fn sl1_via_parse_token_alpha_only_scan_still_works() {
    let (_ctx, span) = ctx_and_span("SL1 B");
    let token = parse_token(&mut span.into())
        .expect("SL1 B must be recognized by parse_token's own alpha-only scan");
    assert_eq!(
        token.to_token().into_owned(),
        Token::OpCode(
            Mnemonic::Sl1,
            Some(DataAccess::Register8(Register8::B)),
            None,
            None
        )
    );
}

/// `parse_single_token` (the fused mnemonic+directive dispatch this work
/// added, sharing one wider, digit-inclusive scan between both tables) must
/// recognize `SL1` too - here `word` is the full literal `"SL1"` (digit
/// included), reaching the new `"SL1"` arm in `dispatch_token2` instead of
/// the original `"SL"` arm. Both arms must produce the identical token.
#[test]
fn sl1_via_parse_single_token_shared_scan_also_works() {
    let (_ctx, span) = ctx_and_span("SL1 B");
    let token = parse_single_token(&mut span.into())
        .expect("SL1 B must be recognized by parse_single_token's shared scan too");
    assert_eq!(
        token.to_token().into_owned(),
        Token::OpCode(
            Mnemonic::Sl1,
            Some(DataAccess::Register8(Register8::B)),
            None,
            None
        )
    );
}

/// `SRL8` is a fake instruction (SRL applied to a 16-bit register pair,
/// implemented as several real opcodes) - unlike `SL1`, a bare `SRL8 BC`
/// statement is NOT intercepted by the statement-level label/macro-call
/// heuristic, so this regressed all the way up through `assemble()` (caught
/// by `cpclib-basm`'s `good_fake_instructions4.asm` test) rather than only
/// being reachable via the low-level `parse_token`/`parse_single_token`
/// entry points the way `SL1` is in the tests above.
#[test]
fn srl8_fake_instruction_still_assembles_for_every_register_pair() {
    for reg in ["BC", "DE", "HL", "IX", "IY"] {
        let code = format!("SRL8 {reg}\n");
        assemble(&code).unwrap_or_else(|e| panic!("SRL8 {reg} must assemble: {e}"));
    }
}

/// `SLA`/`SLL`/`SRA`/`SRL` are real 3-letter mnemonics with no trailing
/// digit - they must be unaffected by `SL1` becoming a literal branch too.
#[test]
fn other_shift_rotate_mnemonics_still_assemble() {
    assert!(assemble("SLA B\n").is_ok(), "SLA must still assemble");
    assert!(assemble("SLL B\n").is_ok(), "SLL must still assemble");
    assert!(assemble("SRA B\n").is_ok(), "SRA must still assemble");
    assert!(assemble("SRL B\n").is_ok(), "SRL must still assemble");
}

/// `NOP` lives only in the directive dispatch table (`parse_directive_of_size_3`,
/// `directives.rs`), not the mnemonic table - it must still be found once the
/// mnemonic table (correctly) rejects it and dispatch falls through to the
/// directive table, reusing the same scanned word rather than rescanning.
#[test]
fn nop_still_assembles() {
    let bytes = assemble("nop\n").expect("nop must assemble");
    assert_eq!(bytes, vec![0x00], "{bytes:?}");
}

/// A label whose name looks like a dispatch-table word plus a trailing
/// digit (but isn't actually `SL1`, the one real such alias) must still
/// resolve as an ordinary label, not be mistaken for a mnemonic/directive
/// by the wider, digit-inclusive scan.
#[test]
fn a_digit_suffixed_label_is_never_mistaken_for_a_dispatch_table_entry() {
    let code = "\
    org 0x8000
DB2:
    nop
    jp DB2
";
    let bytes = assemble(code).expect("a label named DB2 must assemble normally");
    // nop ; jp 0x8000
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");
}

/// A hard failure inside a recognized mnemonic's operand (`cut_err`, e.g.
/// `PUSH` with a register that cannot be pushed) must propagate immediately
/// with its own precise location - not be silently discarded as "not a
/// mnemonic after all" in favor of trying the directive table next (which
/// would report a much less useful "line ending expected" at the start of
/// the line instead).
#[test]
fn a_hard_operand_error_is_not_swallowed_in_favor_of_directive_dispatch() {
    let error = assemble("push tg\n").expect_err("push tg is not a valid operand");
    let text = error.to_string();
    assert!(
        !text.contains("Line ending expected"),
        "the PUSH-specific error must not be discarded in favor of a generic \
         directive-dispatch failure: {text}"
    );
}
