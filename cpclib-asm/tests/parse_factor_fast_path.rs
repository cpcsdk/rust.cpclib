//! `parse_factor` (see `cpclib-asm/src/parser/expression.rs`) now tries a
//! byte-class fast path before its full, unmodified `alt()` of ~12
//! branches - but only for byte classes proven unambiguous (a single byte
//! can only ever start one kind of factor), or - for letters `A`-`F`/`a`-`f`
//! - disambiguated via a lookahead (`looks_like_unprefixed_hex_literal`)
//! that exactly predicts whether `positive_number` would succeed, without
//! attempting it. These tests exercise each implemented fast-path class end
//! to end, with the `A`-`F`/`a`-`f` case covered extensively since it's the
//! one class where getting the lookahead wrong would change what parses
//! successfully, not just how fast it fails.

use cpclib_asm::assemble;

/// Digit-leading and `+`-leading factors (the fast path's `positive_number`
/// class).
#[test]
fn digit_and_plus_leading_numbers_still_assemble() {
    assert_eq!(assemble("db 5\n").unwrap(), vec![5]);
    assert_eq!(assemble("db +5\n").unwrap(), vec![5]);
}

/// `"`-leading string/char literals (the fast path's `parse_string` class).
#[test]
fn quoted_strings_and_chars_still_assemble() {
    assert_eq!(assemble("db \"A\"\n").unwrap(), vec![b'A']);
    assert_eq!(assemble("db \"AB\"\n").unwrap(), vec![b'A', b'B']);
}

/// `_`-leading labels (the fast path's proximity-label/label class).
#[test]
fn underscore_leading_labels_still_assemble() {
    let code = "\
    org 0x8000
_foo:
    nop
    jp _foo
";
    let bytes = assemble(code).expect("a label starting with _ must assemble");
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");
}

/// `(`-leading parenthesized sub-expressions (the fast path's `parens`
/// class).
#[test]
fn parenthesized_expressions_still_assemble() {
    let bytes = assemble("db (1+2)*3\n").expect("a parenthesized expression must assemble");
    assert_eq!(bytes, vec![9], "{bytes:?}");
}

/// `[`-leading bracketed lists (the fast path's `parse_expr_bracketed_list`
/// class), still gated by `!is_orgams` exactly as the original branch was.
#[test]
fn bracketed_lists_still_assemble() {
    let code = r#"
        VALS = [1, 2, 3]
        db list_get(VALS, 1)
    "#;
    let bytes = assemble(code).expect("a bracketed list must still assemble");
    assert_eq!(bytes, vec![2], "{bytes:?}");
}

/// Letters `G`-`Z`/`g`-`z` (outside the hex-digit range, the fast path's
/// bool-or-label class) - a label starting with such a letter still
/// assembles.
#[test]
fn labels_starting_with_a_non_hex_letter_still_assemble() {
    let code = "\
    org 0x8000
ZEBRA:
    nop
    jp ZEBRA
";
    let bytes = assemble(code).expect("a label starting with Z must assemble");
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");
}

/// Letters `A`-`F`/`a`-`f`: ambiguous between a label/bool and a
/// prefix-less hex literal with an `h`/`H` suffix (e.g. `FADEh`) - resolved
/// via `looks_like_unprefixed_hex_literal`'s lookahead instead of the full
/// `alt`. Both outcomes must still resolve correctly.
#[test]
fn hex_literal_with_h_suffix_and_a_similarly_shaped_label_both_still_work() {
    // A prefix-less hex literal ending in 'h' - must be the number, not a label.
    let bytes = assemble("db FAh\n").expect("FAh must assemble as the hex value 0xFA");
    assert_eq!(bytes, vec![0xFA], "{bytes:?}");

    // A label that happens to start with a hex-digit letter and isn't
    // h/H-suffixed - must resolve as a label, not fail as an invalid number.
    let code = "\
    org 0x8000
FADE_LABEL:
    nop
    jp FADE_LABEL
";
    let bytes = assemble(code).expect("a label starting with a hex-range letter must assemble");
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");
}

/// `FALSE` starts with `F` - inside the A-F hex-digit range, not the G-Z
/// fast path - so the lookahead must correctly route it to `parse_bool_value`
/// rather than mistaking it for a hex literal (the digit-run stops at `L`,
/// which isn't a hex digit, so the lookahead never even considers a suffix).
#[test]
fn boolean_false_still_assembles_despite_starting_in_the_hex_letter_range() {
    let bytes = assemble("db FALSE\n").expect("FALSE must assemble as the boolean false");
    assert_eq!(bytes, vec![0], "{bytes:?}");
    let bytes = assemble("db TRUE\n").expect("TRUE must assemble as the boolean true");
    assert_eq!(bytes, vec![1], "{bytes:?}");
}

/// Uppercase `H` suffix must be recognized exactly like lowercase `h`.
#[test]
fn uppercase_h_suffix_is_recognized_as_hex() {
    let bytes = assemble("db 0FAH\n").expect("0FAH must assemble as the hex value 0xFA");
    assert_eq!(bytes, vec![0xFA], "{bytes:?}");
}

/// A hex-digit run immediately followed by `h`/`H` but then by another
/// identifier character (`#`, `@`, `_`, or alphanumeric) is NOT a complete
/// hex literal - it's a label that merely starts with hex-looking text plus
/// an embedded 'h'. Matches `number()`'s own outer trailing-character guard
/// (stricter than just "not alphanumeric").
#[test]
fn a_label_with_an_embedded_h_is_not_mistaken_for_a_truncated_hex_literal() {
    let code = "\
    org 0x8000
FADEhello:
    nop
    jp FADEhello
";
    let bytes =
        assemble(code).expect("a label with an embedded h after hex-looking text must assemble");
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");

    let code = "\
    org 0x8000
FADEh_SUFFIX:
    nop
    jp FADEh_SUFFIX
";
    let bytes = assemble(code)
        .expect("a label with hex-looking text, h, then an underscore must assemble");
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");
}

/// A hex-digit run with no `h`/`H` suffix at all (a plain label starting
/// with hex-range letters and digits) must resolve as a label, matching the
/// pre-existing `FADE_LABEL` case but exercising underscores *inside* the
/// hex-digit-or-underscore run itself (`hex_digits_and_sep` treats `_` as a
/// valid separator, so the lookahead's scan must not stop early there).
#[test]
fn a_label_that_looks_like_hex_digits_with_separators_but_has_no_suffix_is_still_a_label() {
    let code = "\
    org 0x8000
CAFE_BABE:
    nop
    jp CAFE_BABE
";
    let bytes = assemble(code)
        .expect("a label shaped like underscore-separated hex digits must assemble");
    assert_eq!(bytes, vec![0x00, 0xC3, 0x00, 0x80], "{bytes:?}");
}

/// A hex literal at end-of-input (no trailing character at all) must still
/// be recognized - the lookahead's `bytes.get(i + 1) == None` case.
#[test]
fn hex_literal_at_end_of_input_is_still_recognized() {
    let bytes = assemble("db FAh").expect("FAh at end of input must assemble as 0xFA");
    assert_eq!(bytes, vec![0xFA], "{bytes:?}");
}

/// A single hex digit letter with the suffix immediately following (the
/// shortest possible hex-with-suffix literal) - exercises the lookahead's
/// `i == 0` guard boundary (here `i` becomes 1, not 0, since `A` alone is
/// a valid one-character hex-digit run).
#[test]
fn single_digit_hex_literal_with_suffix_still_assembles() {
    let bytes = assemble("db Ah\n").expect("Ah must assemble as the hex value 0xA");
    assert_eq!(bytes, vec![0x0A], "{bytes:?}");
}
