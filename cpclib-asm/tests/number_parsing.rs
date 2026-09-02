//! `number()` (see `cpclib-asm/src/parser/expression.rs`) used to be
//! `alt((parse_float, parse_value))`, which parsed every plain integer's
//! digits twice (once inside a `parse_float` attempt that backtracked for
//! lack of a following `.`, then again from scratch as the `parse_value`
//! branch). It now scans the integer part once and only continues into
//! float parsing when a `.` genuinely follows. These tests lock down the
//! corner cases that made this easy to get subtly wrong: a bare trailing
//! dot with no fractional digits, and a float that fails its own
//! trailing-character guard and must fall back to a plain integer instead
//! of erroring outright.

use cpclib_asm::preamble::*;
use cpclib_asm::{assemble, parser};
use cpclib_common::winnow::stream::AsBStr;
use cpclib_tokens::ordered_float::OrderedFloat;

fn parse_number(code: &'static str) -> LocatedExpr {
    let ctx = Box::leak(Box::new(ParserContextBuilder::default().build(code)));
    ctx.context_name = Some("TEST".into());
    let span = Z80Span::new_extra(code, &*ctx);
    parser::number(&mut span.into()).unwrap_or_else(|e| panic!("{code:?} must parse: {e}"))
}

/// A bare trailing dot with no fractional digits (`parse_float`'s own
/// fractional part was already optional) must still parse as a float, not
/// as the integer `12` followed by an unconsumed `.`.
#[test]
fn a_bare_trailing_dot_parses_as_a_float() {
    let expr = parse_number("12.");
    assert_eq!(expr.to_expr().into_owned(), Expr::Float(OrderedFloat(12.0)));
}

/// An ordinary float with a fractional part still works.
#[test]
fn a_normal_float_still_parses() {
    let expr = parse_number("12.5");
    assert_eq!(expr.to_expr().into_owned(), Expr::Float(OrderedFloat(12.5)));
}

/// A plain integer (the common case the double-parse fix specifically
/// targets) still parses, and does not spuriously become a float.
#[test]
fn a_plain_integer_still_parses() {
    let expr = parse_number("1234");
    assert_eq!(expr.to_expr().into_owned(), Expr::Value(1234));
}

/// Hex/octal/binary forms, all going through the same shared integer scan,
/// still parse to the right value.
#[test]
fn every_integer_base_still_parses() {
    assert_eq!(parse_number("0x1A").to_expr().into_owned(), Expr::Value(0x1A));
    assert_eq!(parse_number("#1A").to_expr().into_owned(), Expr::Value(0x1A));
    assert_eq!(parse_number("1Ah").to_expr().into_owned(), Expr::Value(0x1A));
    assert_eq!(parse_number("0b0101").to_expr().into_owned(), Expr::Value(0b0101));
    assert_eq!(parse_number("0101b").to_expr().into_owned(), Expr::Value(0b0101));
    assert_eq!(parse_number("0o17").to_expr().into_owned(), Expr::Value(0o17));
}

/// A float with a scientific-notation exponent still parses.
#[test]
fn a_float_with_exponent_still_parses() {
    let expr = parse_number("1.5e2");
    assert_eq!(expr.to_expr().into_owned(), Expr::Float(OrderedFloat(150.0)));
}

/// `12.5x`: a float parse that then fails its own trailing-character guard
/// (`x` immediately follows, no operator/separator between them) must fall
/// back to parsing just the plain integer `12`, leaving `.5x` unconsumed for
/// the rest of the grammar - the same fallback `alt((parse_float,
/// parse_value))` gave before, now that both outcomes share one scan of the
/// integer part instead of `alt` re-invoking two independent scanners.
#[test]
fn a_float_that_fails_its_trailing_guard_falls_back_to_the_integer() {
    let ctx = Box::leak(Box::new(ParserContextBuilder::default().build("12.5x")));
    ctx.context_name = Some("TEST".into());
    let span = Z80Span::new_extra("12.5x", &*ctx);
    let mut input = span.into();
    let expr = parser::number(&mut input).expect("must fall back to the integer 12");
    assert_eq!(expr.to_expr().into_owned(), Expr::Value(12));
    // ".5x" must remain unconsumed - `number()` only ate "12".
    let remaining = std::str::from_utf8(input.as_bstr()).unwrap();
    assert_eq!(remaining, ".5x");
}

/// End-to-end: a bare-dot float used as a real operand still assembles
/// (`DB 12.` must not regress to consuming only `12` and choking on the
/// stray `.`).
#[test]
fn a_bare_dot_float_assembles_end_to_end() {
    let bytes = assemble("db 12.\n").expect("DB 12. must assemble (float truncates to 12)");
    assert_eq!(bytes, vec![12], "{bytes:?}");
}

/// A number immediately followed by a label-starting character must still
/// be rejected as a number (both outcomes' trailing-character guard is
/// unchanged) and fall through to whatever else the grammar does with it.
#[test]
fn a_number_immediately_followed_by_a_letter_is_not_swallowed_as_a_number() {
    // `12ABC` as a whole must not be treated as the number 12 - it should
    // either fail to parse as `number` at all, or (in context) be parsed as
    // something else entirely. Here we just confirm assembling code that
    // relies on `12` and a following label being properly separated still
    // works, and that a glued-together `12ABC`-shaped label reference
    // assembles as a label, not a truncated number.
    let code = "\
    org 0x8000
12ABC:
    nop
";
    // Whether or not `12ABC` is accepted as a label name is a separate
    // grammar question; what matters here is that `number()` alone
    // (exercised directly) rejects it as a number rather than silently
    // truncating to 12.
    let ctx = Box::leak(Box::new(ParserContextBuilder::default().build("12ABC")));
    ctx.context_name = Some("TEST".into());
    let span = Z80Span::new_extra("12ABC", &*ctx);
    let res = parser::number(&mut span.into());
    assert!(res.is_err(), "12ABC must not parse as a bare number: {res:?}");
    let _ = code; // documents the broader context this guard exists for
}
