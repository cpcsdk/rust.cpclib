//! Regression tests for panics fixed in the tokenized-BASIC binary parser:
//! the `TryFrom<u8>` impls for `BasicTokenNoPrefix`/`BasicTokenPrefixed`
//! deliberately reject certain byte values, but the parser used to
//! `.unwrap()` the result anyway (throwing away that error handling), and a
//! line whose declared length was 1-3 used to underflow a subtraction
//! before the resulting buffer was even sliced. All of these are directly
//! reachable from a corrupted or adversarial `.bas`-style binary program -
//! `BasicProgram::decode` must return `Err`, not panic.

use cpclib_basic::{BasicError, BasicProgram};

/// Byte 5 is explicitly rejected by `BasicTokenNoPrefix::try_from`.
const INVALID_NO_PREFIX_TOKEN: u8 = 5;

fn line(line_number: u16, body: &[u8]) -> Vec<u8> {
    // length (u16 LE, includes itself + line_number + body) + line_number (u16 LE) + body
    let length = (4 + body.len()) as u16;
    let mut out = Vec::new();
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&line_number.to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn end_marker() -> Vec<u8> {
    0u16.to_le_bytes().to_vec()
}

#[test]
fn an_invalid_token_byte_is_a_parse_error_not_a_panic() {
    // body: one invalid token byte, then the mandatory trailing 0.
    let mut bytes = line(10, &[INVALID_NO_PREFIX_TOKEN, 0]);
    bytes.extend(end_marker());

    let result = BasicProgram::decode(&bytes);
    assert!(
        matches!(result, Err(BasicError::ParseError { .. })),
        "expected a ParseError, got {result:?}"
    );
}

#[test]
fn a_line_length_of_one_does_not_underflow() {
    // length = 1: too short to even hold the line-number field, let alone
    // `length - 4` underflowing before the trailing-byte check.
    let mut bytes = 1u16.to_le_bytes().to_vec();
    bytes.extend(end_marker());

    let result = BasicProgram::decode(&bytes);
    assert!(
        matches!(result, Err(BasicError::ParseError { .. })),
        "expected a ParseError, got {result:?}"
    );
}

#[test]
fn a_line_length_of_three_does_not_underflow() {
    let mut bytes = 3u16.to_le_bytes().to_vec();
    bytes.extend(end_marker());

    let result = BasicProgram::decode(&bytes);
    assert!(
        matches!(result, Err(BasicError::ParseError { .. })),
        "expected a ParseError, got {result:?}"
    );
}

#[test]
fn a_well_formed_empty_program_still_decodes() {
    // Sanity check: the above fixes must not have broken the trivial
    // well-formed case (just the end-of-program marker, no lines).
    let bytes = end_marker();
    let result = BasicProgram::decode(&bytes);
    assert!(result.is_ok(), "expected Ok, got {result:?}");
}
