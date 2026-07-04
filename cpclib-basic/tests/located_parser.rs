use cpclib_basic::located::LocatedBasicProgram;

const HBL_INTRO: &str = r#"10 INK 3,26,26
11 PEN 3:PRINT "   NO ";:PEN 1:PRINT " hardware sprites!"
12 PEN 3:PRINT "+  NO ";:PEN 1:PRINT " programmable interrupts!"
12 PEN 3:PRINT "+  NO ";:PEN 1:PRINT " char-mode!"
12 PEN 3:PRINT "+  NO ";:PEN 1:PRINT " multi-resolution per line!"
13 PEN 3:PRINT "+ ONLY ";:PEN 1:PRINT "Z80 and CRTC skills"
14 PRINT ""
20 PEN 3:PRINT "H";:PEN 1: PRINT "orny ";
21 PEN 3:PRINT "B";:PEN 1: PRINT "ytes ";
22 PEN 3:PRINT "L";:PEN 1: PRINT "overs"
30 PEN 1:PRINT "                  by ";
40 PEN 3:PRINT "Benediction ";
50 PEN 1:PRINT CHR$(13);:PRINT CHR$(10);:PRINT CHR$(10);:PRINT CHR$(10);
60 PRINT "run";
70 PRINT CHR$(34);:PRINT "HBL";
80 PRINT CHR$(&0d);:PRINT CHR$(&0b);"#;

#[test]
fn test_located_parse_hbl_intro() {
    let prog = LocatedBasicProgram::parse(HBL_INTRO)
        .expect("LocatedBasicProgram::parse should not fail");
    // We have 16 source lines, all with valid line numbers.
    // Duplicate line 12 appears three times — all should be parsed.
    assert!(
        prog.lines.len() >= 13,
        "expected at least 13 parsed lines, got {}",
        prog.lines.len()
    );
}

#[test]
fn test_located_tokens_not_empty() {
    let prog = LocatedBasicProgram::parse(HBL_INTRO).unwrap();
    for bline in &prog.lines {
        assert!(
            !bline.tokens.is_empty(),
            "line {} should have at least one token",
            bline.line_number
        );
    }
}

#[test]
fn test_located_line_numbers_correct() {
    let prog = LocatedBasicProgram::parse(HBL_INTRO).unwrap();
    let nums: Vec<u16> = prog.lines.iter().map(|l| l.line_number).collect();
    // Line numbers must start with 10, 11, 12, 12, 12, 13, ...
    assert_eq!(nums[0], 10);
    assert_eq!(nums[1], 11);
    assert_eq!(nums[2], 12);
    assert_eq!(nums[3], 12);
    assert_eq!(nums[4], 12);
}

#[test]
fn test_located_hex_literal() {
    // CHR$(&0d) and CHR$(&0b) must parse without skipping the line.
    let prog = LocatedBasicProgram::parse(HBL_INTRO).unwrap();
    let last = prog.lines.last().expect("should have at least one line");
    assert_eq!(last.line_number, 80);
}

#[test]
fn test_located_simple_goto() {
    let src = "10 GOTO 20\n20 END";
    let prog = LocatedBasicProgram::parse(src).unwrap();
    assert_eq!(prog.lines.len(), 2);
}

#[test]
fn test_located_for_next() {
    let src = "10 FOR I=1 TO 10\n20 NEXT I";
    let prog = LocatedBasicProgram::parse(src).unwrap();
    assert_eq!(prog.lines.len(), 2);
    use cpclib_basic::located::LocatedTokenKind;
    use cpclib_basic::tokens::BasicTokenNoPrefix;
    let has_for = prog.lines[0]
        .tokens
        .iter()
        .any(|t| matches!(&t.kind, LocatedTokenKind::Keyword(BasicTokenNoPrefix::For)));
    assert!(has_for, "line 10 should contain FOR keyword token");
}
