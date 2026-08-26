use cpclib_basic::BasicProgram;

/// Helper function to test round-trip conversion: source → bytes → source
fn test_roundtrip(original_code: &str) {
    // Parse the original source code
    let prog =
        BasicProgram::parse(original_code).expect(&format!("Failed to parse: {}", original_code));

    let original_output = prog.to_string();

    // Convert to bytes
    let bytes = prog.as_bytes();

    // Decode back from bytes
    let decoded_prog = BasicProgram::decode(&bytes)
        .expect(&format!("Failed to decode bytes for: {}", original_code));

    let decoded_output = decoded_prog.to_string();

    // Convert back to bytes again to ensure stability
    let bytes2 = decoded_prog.as_bytes();

    // The outputs should be identical
    assert_eq!(
        original_output.trim(),
        decoded_output.trim(),
        "Round-trip failed for: {}\nOriginal: {}\nDecoded:  {}",
        original_code,
        original_output,
        decoded_output
    );

    // The bytes should be identical
    assert_eq!(
        bytes, bytes2,
        "Byte encoding is not stable for: {}",
        original_code
    );
}

#[test]
fn roundtrip_simple_print() {
    test_roundtrip("10 PRINT \"HELLO\"");
}

#[test]
fn roundtrip_print_with_space() {
    test_roundtrip("10 PRINT \" HELLO \"");
}

#[test]
fn roundtrip_print_multiple_strings() {
    test_roundtrip("10 PRINT \"HELLO\";\"WORLD\"");
}

#[test]
fn roundtrip_print_with_semicolon() {
    test_roundtrip("10 PRINT \"HELLO\";");
}

#[test]
fn roundtrip_print_no_bug() {
    // This is the specific case that was reported as buggy
    test_roundtrip("10 PRINT \" NO\";");
}

#[test]
fn roundtrip_print_empty_string() {
    test_roundtrip("10 PRINT \"\"");
}

#[test]
fn roundtrip_print_with_comma() {
    test_roundtrip("10 PRINT \"A\",\"B\"");
}

#[test]
fn roundtrip_print_string_and_number() {
    test_roundtrip("10 PRINT \"VALUE=\";42");
}

#[test]
fn roundtrip_print_multiple_spaces() {
    test_roundtrip("10 PRINT \"  SPACE  \"");
}

#[test]
fn roundtrip_print_special_chars() {
    test_roundtrip("10 PRINT \"!@#$%\"");
}

#[test]
fn roundtrip_multiple_lines_with_print() {
    test_roundtrip("10 PRINT \"LINE1\"\n20 PRINT \"LINE2\"\n30 PRINT \"LINE3\"");
}

#[test]
fn roundtrip_print_with_expression() {
    test_roundtrip("10 PRINT \"X=\";X");
}

#[test]
fn roundtrip_complex_print() {
    test_roundtrip("10 PRINT \"A\";1;\"B\";2;\"C\"");
}

#[test]
fn roundtrip_print_tab() {
    test_roundtrip("10 PRINT TAB(10);\"TEXT\"");
}

#[test]
fn roundtrip_print_newline() {
    test_roundtrip("10 PRINT \"LINE1\":PRINT \"LINE2\"");
}

#[test]
fn roundtrip_input_with_string() {
    test_roundtrip("10 INPUT \"ENTER VALUE\";A");
}

#[test]
fn roundtrip_if_with_string() {
    test_roundtrip("10 IF A$=\"YES\" THEN PRINT \"OK\"");
}

#[test]
fn roundtrip_string_assignment() {
    test_roundtrip("10 A$=\"HELLO\"");
}

#[test]
fn roundtrip_string_concatenation() {
    test_roundtrip("10 A$=\"HELLO\"+\"WORLD\"");
}

#[test]
fn roundtrip_print_using() {
    test_roundtrip("10 PRINT USING \"##.##\";3.14");
}

#[test]
fn roundtrip_rem_with_quotes() {
    test_roundtrip("10 REM \"THIS IS A COMMENT\"");
}

#[test]
fn roundtrip_data_with_strings() {
    test_roundtrip("10 DATA \"APPLE\",\"ORANGE\",\"BANANA\"");
}

#[test]
fn roundtrip_multiline_complex() {
    let code = "10 PRINT \"START\"\n\
                20 INPUT \"NAME\";N$\n\
                30 PRINT \"HELLO \";N$\n\
                40 PRINT \"END\"";
    test_roundtrip(code);
}

#[test]
fn roundtrip_quoted_semicolon() {
    // Test various string patterns with semicolons
    test_roundtrip("10 PRINT \";\";");
    test_roundtrip("10 PRINT \"A;\";");
    test_roundtrip("10 PRINT \";B\"");
    test_roundtrip("10 PRINT \"A;B\";");
}

#[test]
fn roundtrip_quoted_comma() {
    // Test various string patterns with commas
    test_roundtrip("10 PRINT \",\";");
    test_roundtrip("10 PRINT \"A,\";");
    test_roundtrip("10 PRINT \",B\"");
    test_roundtrip("10 PRINT \"A,B\",");
}

#[test]
/// Regression test, reported live: decoding a program back to text doubled
/// every space around AND/OR/XOR/MOD - `Display` for these tokens used to
/// pad itself with its own leading/trailing space on top of the real space
/// *already* separately stored around them (confirmed against a real CPC's
/// own saved bytes, `mandelbrot_matches_a_real_cpcs_own_save`). `NOT` is
/// included here too but for the opposite reason: it's parsed as a unary
/// prefix that only peeks at (doesn't store) the character after it, so it
/// genuinely needs its own trailing space and must NOT lose it the same way
/// - this test pins both directions down together. `test_roundtrip` alone
/// doesn't catch either - it only checks the two `Display` calls agree with
/// *each other*, not that either matches the spacing actually typed.
fn roundtrip_and_or_xor_mod_not_do_not_double_space() {
    let code = "10 IF x2+y2<=4 AND it<itmax THEN y=1\n\
                20 IF a OR b THEN y=2\n\
                30 IF a XOR b THEN y=3\n\
                40 c=a MOD b\n\
                50 IF NOT a THEN y=5";
    let prog = BasicProgram::parse(code).expect("parse");
    let bytes = prog.as_bytes();
    let decoded = BasicProgram::decode(&bytes).expect("decode");
    assert_eq!(
        decoded.to_string().trim(),
        code.trim(),
        "decoded spacing must match what was actually typed"
    );
}

#[test]
/// Regression test, reported live: `IF ... THEN 100` (a bare line number,
/// Locomotive BASIC's implicit-GOTO shorthand) decoded back as
/// `THEN GOTO100` - a missing space and a wrongly-spelled "GOTO" (one word)
/// where a real ROM's own LIST prints "GO TO" (two words). The parser
/// synthesises a real `Goto` token for this shape with nothing stored
/// after it (confirmed against a real CPC's own saved bytes,
/// `mandelbrot_matches_a_real_cpcs_own_save`, that a keyboard-typed
/// program's own tokenised bytes carry exactly this token, the same as an
/// *explicit* `GOTO 80` does). What a real ROM's own LIST renders it as
/// was checked live against 1984js - a true ROM emulator, not a
/// reimplementation - executing this crate's own tokenised bytes directly:
/// "GO TO 100", two words with a space, never "GOTO100" and never a bare
/// "100" (AMSpiriT Lite's own `POST /api/basic` endpoint produces
/// genuinely different bytes for the same source - a bare `LineNumber`
/// token with no `Goto` at all - which is a different, also-valid
/// encoding, not evidence about what *this* crate's own bytes render as).
/// Covers the same shorthand after `ELSE` too, and confirms an *explicit*
/// `GOTO 80` is unaffected either way.
fn roundtrip_implicit_then_and_else_linenumber_renders_as_go_to() {
    let code = "10 IF a THEN 100\n\
                20 IF b THEN 200 ELSE 300\n\
                30 IF c THEN y=1:GOTO 80";
    let expected = "10 IF a THEN GO TO 100\n\
                    20 IF b THEN GO TO 200 ELSE GO TO 300\n\
                    30 IF c THEN y=1:GOTO 80";
    let prog = BasicProgram::parse(code).expect("parse");
    let bytes = prog.as_bytes();
    let decoded = BasicProgram::decode(&bytes).expect("decode");
    assert_eq!(
        decoded.to_string().trim(),
        expected,
        "an implicit THEN/ELSE line number must render as GO TO (two words), matching a real ROM's own LIST"
    );
}

#[test]
fn roundtrip_edge_cases() {
    // Single character strings
    test_roundtrip("10 PRINT \"A\"");
    test_roundtrip("10 PRINT \"1\"");
    test_roundtrip("10 PRINT \" \"");

    // Multiple consecutive strings
    test_roundtrip("10 PRINT \"A\";\"B\";\"C\";\"D\"");

    // Mix of quoted and unquoted
    test_roundtrip("10 PRINT A;\"B\";C");
}
