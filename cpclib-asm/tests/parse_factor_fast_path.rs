//! `parse_factor` (see `cpclib-asm/src/parser/expression.rs`) now tries a
//! byte-class fast path before its full, unmodified `alt()` of ~12
//! branches - but only for byte classes proven unambiguous (a single byte
//! can only ever start one kind of factor). These tests exercise each
//! implemented fast-path class end to end, plus the boundary cases the
//! fast path deliberately leaves to the full `alt` untouched (letters
//! `A`-`F`/`a`-`f`, which are ambiguous between a label and a prefix-less
//! hex literal with an `h` suffix).

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

/// Boundary case deliberately left OUT of the fast path: letters `A`-`F`/
/// `a`-`f` are ambiguous between a label and a prefix-less hex literal with
/// an `h`/`H` suffix (e.g. `FADEh`) - both must still resolve correctly
/// through the untouched, full `alt`.
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
