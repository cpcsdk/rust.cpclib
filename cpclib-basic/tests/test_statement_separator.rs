//! Regression tests for the `:` statement-separator byte value.
//!
//! Real AMSDOS-tokenized BASIC uses a dedicated byte `0x01` for a `:` used
//! as a statement separator - distinct from the generic ASCII-printable
//! range's literal `:` (`0x3A`), used when a colon appears as *data* inside
//! a string literal or a REM comment. See
//! `cpclib-basic/tests/Technical information about Locomotive BASIC -
//! CPCWiki.htm`'s "BASIC tokens" table (`01 ":" statement seperator`).
//!
//! Follows the same byte-level-assertion idiom as `test_original_bug.rs`,
//! with one caveat found while writing these tests: a raw "count every
//! `0x01` byte in the whole output" check is unreliable on its own, since
//! `0x01` also legitimately appears as the low byte of an encoded 16-bit
//! integer constant whose value happens to be 1 (e.g. `PEN 1` encodes as
//! `1A 01 00`). The site-specific tests below therefore pick argument
//! values that can't alias with `0x01`, and the real-file test counts
//! `BasicToken::is_statement_separator()` at the token level instead of
//! scanning raw bytes, which has no such ambiguity.

use cpclib_basic::BasicProgram;
use cpclib_basic::tokens::BasicTokenNoPrefix;

const STATEMENT_SEPARATOR: u8 = BasicTokenNoPrefix::StatementSeparator as u8;
const CHAR_COLON: u8 = b':'; // 0x3A - real ASCII value, used for data-position colons

fn statement_separator_token_count(prog: &BasicProgram) -> usize {
    prog.lines()
        .iter()
        .flat_map(|l| l.tokens())
        .filter(|t| t.is_statement_separator())
        .count()
}

#[test]
fn statement_separator_plain_inter_statement() {
    let code = "10 PEN 8:PRINT \"H\"\n";
    let prog = BasicProgram::parse(code).unwrap();
    let bytes = prog.as_bytes();
    println!("{code:?} -> {bytes:02X?}");

    assert_eq!(statement_separator_token_count(&prog), 1);
    let byte_count = bytes.iter().filter(|&&b| b == STATEMENT_SEPARATOR).count();
    assert_eq!(
        byte_count, 1,
        "expected exactly one 0x01 statement-separator byte, got {bytes:02X?}"
    );

    let decoded = BasicProgram::decode(&bytes).unwrap();
    assert_eq!(prog.to_string().trim(), decoded.to_string().trim());
}

#[test]
fn statement_separator_leading_colon() {
    let code = "10 :PRINT \"x\"\n";
    let prog = BasicProgram::parse(code).unwrap();
    let bytes = prog.as_bytes();
    println!("{code:?} -> {bytes:02X?}");

    assert_eq!(statement_separator_token_count(&prog), 1);
    let byte_count = bytes.iter().filter(|&&b| b == STATEMENT_SEPARATOR).count();
    assert_eq!(
        byte_count, 1,
        "leading ':' should encode as one 0x01 byte, got {bytes:02X?}"
    );
}

#[test]
fn statement_separator_if_then_compound() {
    // PEN 8/7/6 rather than 1/2/3 - a literal integer constant's encoded
    // low byte would otherwise alias with the 0x01 separator byte itself
    // (see module doc comment), which would make a raw byte-count assertion
    // meaningless here.
    let code = "10 IF 5=5 THEN PEN 8:PEN 7:PEN 6\n";
    let prog = BasicProgram::parse(code).unwrap();
    let bytes = prog.as_bytes();
    println!("{code:?} -> {bytes:02X?}");

    assert_eq!(
        statement_separator_token_count(&prog),
        2,
        "expected 2 statement-separator tokens in a 3-statement THEN clause"
    );
    let byte_count = bytes.iter().filter(|&&b| b == STATEMENT_SEPARATOR).count();
    assert_eq!(
        byte_count, 2,
        "expected 2 statement-separator bytes, got {bytes:02X?}"
    );
}

#[test]
fn statement_separator_if_else_compound() {
    let code = "10 IF 5=6 THEN PEN 8 ELSE PEN 7:PEN 6\n";
    let prog = BasicProgram::parse(code).unwrap();
    let bytes = prog.as_bytes();
    println!("{code:?} -> {bytes:02X?}");

    assert_eq!(
        statement_separator_token_count(&prog),
        1,
        "expected 1 statement-separator token in the ELSE clause"
    );
    let byte_count = bytes.iter().filter(|&&b| b == STATEMENT_SEPARATOR).count();
    assert_eq!(
        byte_count, 1,
        "expected 1 statement-separator byte, got {bytes:02X?}"
    );
}

#[test]
fn literal_colon_in_string_still_encodes_as_ascii_colon() {
    // Regression guard: a ':' *inside* a string literal must remain 0x3A,
    // never become the 0x01 statement-separator byte - proves the fix is
    // scoped to real statement separators, not every ':' character.
    let code = "10 PRINT \"A:B\"\n";
    let prog = BasicProgram::parse(code).unwrap();
    let bytes = prog.as_bytes();
    println!("{code:?} -> {bytes:02X?}");

    assert_eq!(statement_separator_token_count(&prog), 0);
    assert!(
        bytes.windows(3).any(|w| w == [b'A', CHAR_COLON, b'B']),
        "literal ':' inside a string should stay 0x3A, got {bytes:02X?}"
    );
    assert!(
        !bytes.contains(&STATEMENT_SEPARATOR),
        "a string-literal colon must never produce a 0x01 byte, got {bytes:02X?}"
    );
}

#[test]
fn literal_colon_in_rem_still_encodes_as_ascii_colon() {
    let code = "10 REM A:B\n";
    let prog = BasicProgram::parse(code).unwrap();
    let bytes = prog.as_bytes();
    println!("{code:?} -> {bytes:02X?}");

    assert_eq!(statement_separator_token_count(&prog), 0);
    assert!(
        bytes.windows(3).any(|w| w == [b'A', CHAR_COLON, b'B']),
        "literal ':' inside a REM comment should stay 0x3A, got {bytes:02X?}"
    );
    assert!(
        !bytes.contains(&STATEMENT_SEPARATOR),
        "a REM-comment colon must never produce a 0x01 byte, got {bytes:02X?}"
    );
}

/// End-to-end verification against the real repro file that originally
/// motivated this fix (heavy `PRINT`/`CHR$`/`:`/`;` usage, e.g.
/// `1000 PEN 2:PRINT "H";:PEN 3:PRINT "orny ";`). Vendored from
/// `cpclib-lsp/tests/fixtures/CATART2.ASC` (verified byte-identical to the
/// original at `demo.bnd5/linking/data/CATART2.ASC`), mirroring that
/// crate's own precedent for keeping test fixtures in-repo.
///
/// Deliberately does *not* assert a raw "count every 0x01 byte" total
/// against the token count here - this file's many `CHR$(&a)`/`CHR$(&b)`/
/// `PEN 2`/etc. numeric literals legitimately encode bytes that alias with
/// `0x01` (see module doc comment), so only the token-level count (which
/// has no such ambiguity) and the decode round-trip are asserted.
#[test]
fn catart2_asc_encodes_and_decodes_with_the_expected_separator_count() {
    let source = include_str!("fixtures/CATART2.ASC");
    let prog = BasicProgram::parse(source).expect("CATART2.ASC must still parse cleanly");
    let bytes = prog.as_bytes();

    let token_level_count = statement_separator_token_count(&prog);
    assert!(
        token_level_count > 0,
        "CATART2.ASC is known to contain ':'-joined statements"
    );

    let decoded = BasicProgram::decode(&bytes).expect("must decode back cleanly");
    let decoded_count = statement_separator_token_count(&decoded);
    assert_eq!(
        decoded_count, token_level_count,
        "the statement-separator count must survive an encode -> decode round trip"
    );
    assert_eq!(prog.to_string().trim(), decoded.to_string().trim());
}
