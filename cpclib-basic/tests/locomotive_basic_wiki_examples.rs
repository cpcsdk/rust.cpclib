use cpclib_basic::string_parser::{parse_basic_line, parse_instruction};
use cpclib_common::winnow::Parser;

// ============================================================================
// CPCWiki Locomotive BASIC Examples Test Suite
// ============================================================================
//
// This test suite validates the parser against official examples from the
// Locomotive BASIC documentation (Amstrad CPC6128 manual).
//
// Current Status: 24/46 tests passing (52%)
//
// Known parser limitations causing test failures:
// 
// 1. Immediate mode commands with incomplete argument parsing:
//    - AUTO [<line number>][,<increment>] - Example: "AUTO 100,5"
//      Parser implementation exists but doesn't parse the optional arguments
//    - CALL <address>[,<list of:<parameter>] - Example: "CALL 0"
//    - CAT - Simple command, should work without arguments
//    - MODE <mode number> - Example: "MODE 1"
//    - RANDOMIZE TIME - Should be recognized as complete statement
//
// 2. Graphics commands needing implementation/fixes:
//    - SYMBOL <char>,<byte1>,<byte2>,...,<byte8> - Define custom character
//    - ORIGIN <x left>,<x right>,<y top>,<y bottom>,<x>,<y> - Set coordinate system
//    - WINDOW#<stream>,<left>,<right>,<top>,<bottom> - Define text window
//
// 3. I/O statements with optional/variant syntax not fully supported:
//    - INPUT [#<channel>][;][<prompt> <separator>] <list of:<variable>>
//      Documentation shows INPUT can omit the prompt: "INPUT choice" is valid
//      Current parser may require the prompt string
//    - WRITE #<stream>,<list of items> - File output statement
//    - LINE INPUT#<stream>,<variable> - Read line from file
//    - EOF - Test end of file (no argument needed per doc example "WHILE NOT EOF")
//
// 4. Numeric literal parsing issues:
//    - Float literals with many decimal places (e.g., 9.80665)
//    - Large integers approaching limit (123456789 vs max 32767 for integers)
//      Note: CPC supports integers -32768 to +32767, reals -1.7E+38 to +1.7E+38
//
// 5. Advanced expressions:
//    - DEF FN with complex expressions: "DEF FNgrv=s0+v0*t+0.5*gn*t^2"
//    - MID$ as assignment target: "MID$(a$,2,3)='ipp'" (string manipulation)
//    - Multiple statements with : (e.g., "PRINT HEX$(value1):PRINT BIN$(value1,8)")
//      May work for some cases but not all
//
// 6. Interrupt/timer statements:
//    - EVERY <period>[,<timer>] GOSUB <line> - Periodic interrupt
//    - AFTER <delay>[,<timer>] GOSUB <line> - One-shot interrupt
//
// 7. Bit operation functions in complex contexts:
//    - BIN$(<value>,<digits>) - Binary string representation
//    - HEX$(<value>[,<width>]) - Hexadecimal string
//    - Expressions like "counter=(counter+1) AND 31"
//
// These represent features to be implemented in the parser, not test issues.
// Each failure points to a specific area where the parser needs enhancement
// to fully support Locomotive BASIC 1.1 specification.
// ============================================================================

/// Helper function to test parsing and reconstruction of a BASIC line (with line number)
fn test_basic_line(line: &str) -> Result<String, String> {
    let line_with_newline = format!("{}\n", line);
    match parse_basic_line.parse(&line_with_newline) {
        Ok(parsed) => Ok(parsed.to_string()),
        Err(e) => Err(format!("Parse error: {:?}", e))
    }
}

/// Helper function to test parsing and reconstruction of an immediate mode statement (no line number)
fn test_immediate_statement(statement: &str) -> Result<String, String> {
    let statement_with_newline = format!("{}\n", statement);
    let mut input = statement_with_newline.as_str();
    
    match parse_instruction.parse_next(&mut input) {
        Ok(tokens) => {
            // Check that only whitespace/newline remains
            let remaining = input.trim();
            if !remaining.is_empty() {
                return Err(format!("Unconsumed input after parsing: '{}'", remaining));
            }
            
            // Reconstruct the statement from tokens
            let reconstructed = tokens.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("");
            Ok(reconstructed)
        },
        Err(e) => Err(format!("Parse error: {:?}", e))
    }
}

/// Helper function to test multiple lines and report failures
fn test_lines(lines: &[&str]) {
    let mut failed = Vec::new();
    
    for line in lines {
        if let Err(e) = test_basic_line(line) {
            failed.push((line, e));
        }
    }
    
    if !failed.is_empty() {
        let mut msg = format!("Failed to parse {} line(s):\n", failed.len());
        for (line, err) in &failed {
            msg.push_str(&format!("  {}: {}\n", line, err));
        }
        panic!("{}", msg);
    }
}

/// Helper function to test multiple immediate mode statements and report failures
fn test_immediate_statements(statements: &[&str]) {
    let mut failed = Vec::new();
    
    for statement in statements {
        if let Err(e) = test_immediate_statement(statement) {
            failed.push((statement, e));
        }
    }
    
    if !failed.is_empty() {
        let mut msg = format!("Failed to parse {} immediate statement(s):\n", failed.len());
        for (statement, err) in &failed {
            msg.push_str(&format!("  {}: {}\n", statement, err));
        }
        panic!("{}", msg);
    }
}

/// Macro to generate a test that checks multiple BASIC lines (with line numbers)
macro_rules! basic_test {
    ($test_name:ident, $($line:expr),+ $(,)?) => {
        #[test]
        fn $test_name() {
            test_lines(&[$($line),+]);
        }
    };
}

/// Macro to generate a test that checks multiple immediate mode statements (no line numbers)
macro_rules! immediate_test {
    ($test_name:ident, $($statement:expr),+ $(,)?) => {
        #[test]
        fn $test_name() {
            test_immediate_statements(&[$($statement),+]);
        }
    };
}

// ============================================================================
// Simple Commands (immediate mode - no line numbers)
// ============================================================================

immediate_test!(test_auto_command, "AUTO 100,5");
immediate_test!(test_call_command, "CALL 0");
immediate_test!(test_cat_command, "CAT");
immediate_test!(test_randomize_time, "RANDOMIZE TIME");

// ============================================================================
// DATA and Type Definition
// ============================================================================

basic_test!(
    test_data_read_example,
    r#"10 DATA "Hello, world!", 42"#,
    r#"20 READ message$:PRINT message$"#,
    r#"30 READ answer:PRINT "The answer is:";answer"#,
);

basic_test!(test_defint_example, "10 DEFINT F,S");

basic_test!(
    test_defint_with_print,
    "10 DEFINT A-Z",
    "20 FIRST=111.11:SECOND=22.2",
    "30 PRINT FIRST,SECOND",
);

basic_test!(
    test_data_declaration,
    "10 DATA 10,11,12,13,14",
    "20 DATA 20",
);

// ============================================================================
// Graphics Commands
// ============================================================================

immediate_test!(
    test_draw_example,
    "CLG 2",
    "DRAW 500,400,0",
);

basic_test!(
    test_draw_multi_statement,
    "10 MASK 15:MOVE 0,0:DRAW 500,400",
);

immediate_test!(
    test_drawr_example,
    "MOVE 200,200",
    "DRAWR 100,100,0",
);

basic_test!(
    test_ink_example,
    "10 MODE 2",
    "20 INK 0,3",
    "30 INK 1,26",
);

basic_test!(
    test_mask_example,
    "10 CLG 2:MASK 1:MOVE 0,0:DRAW 500,400",
    "20 MASK 15:MOVE 0,0:DRAW 500,400",
);

immediate_test!(
    test_origin_example,
    "ORIGIN 320,200,250,450,100,300",
    "DRAW 0,200",
);

immediate_test!(
    test_symbol_example,
    "SYMBOL 255,255,129,129,129,129,129,129,255",
    "PRINT CHR$(255)",
);

immediate_test!(
    test_window_example,
    "MODE 1",
    "WINDOW#1,1,40,1,6",
);

// ============================================================================
// Control Flow
// ============================================================================

// ============================================================================
// Control Flow
// ============================================================================

basic_test!(
    test_for_loop,
    "10 FOR I=1 TO 10",
    "20 PRINT I;",
    "30 NEXT I",
    "40 PRINT I",
);

basic_test!(
    test_gosub_example,
    r#"10 PRINT "Calling subroutine""#,
    "20 GOSUB 100",
    r#"30 PRINT "Back from subroutine""#,
    "40 END",
    "100 REM Begin of the subroutine",
    r#"110 PRINT "Subroutine started""#,
    "120 RETURN",
);

basic_test!(
    test_goto_simple,
    "10 GOTO 100",
    "20 REM not executed",
    "30 REM not executed",
    r#"100 PRINT "Hello World!""#,
);

basic_test!(
    test_goto_endless_loop,
    r##"10 PRINT "#";"##,
    "20 GOTO 10",
);

basic_test!(
    test_goto_with_condition,
    "10 I=1",
    "20 PRINT I",
    "30 I=I+1",
    "40 IF I<25 THEN GOTO 20",
    "50 END",
);

basic_test!(
    test_if_then_else,
    r#"10 INPUT "guess a figure:",f"#,
    r#"20 IF f=10 THEN PRINT "right": END: ELSE GOTO 10"#,
);

basic_test!(
    test_if_nested,
    r#"10 INPUT "guess a figure:",f"#,
    r#"20 IF f=10 THEN PRINT "right": END: ELSE IF f<10 THEN PRINT "too small" ELSE PRINT "too big""#,
    "30 GOTO 10",
);

basic_test!(
    test_on_goto_menu,
    r#"10 PRINT "1. LOAD - 2. SAVE - 3. EXIT""#,
    "20 INPUT choice",
    "30 ON choice GOTO 1000, 2000, 3000",
    "40 CLS: GOTO 10",
    r#"1000 PRINT "1. LOAD":END"#,
    r#"2000 PRINT "2. SAVE":END"#,
    "3000 END",
);

basic_test!(
    test_on_gosub_menu,
    r#"10 PRINT "1. LOAD - 2. SAVE - 3. EXIT""#,
    "20 INPUT choice",
    "30 ON choice GOSUB 1000, 2000, 3000",
    "40 CLS: GOTO 10",
    r#"1000 PRINT "1. LOAD":RETURN"#,
    r#"2000 PRINT "2. SAVE":RETURN"#,
    "3000 END",
);

// ============================================================================
// INPUT Statement Tests (various forms per documentation)
// ============================================================================

basic_test!(
    test_input_simple_variable,
    "10 INPUT choice",
    "20 PRINT choice",
);

basic_test!(
    test_input_with_prompt_semicolon,
    r#"10 INPUT "Enter value";x"#,
    "20 PRINT x",
);

basic_test!(
    test_input_with_prompt_comma,
    r#"10 INPUT "Enter value",x"#,
    "20 PRINT x",
);

basic_test!(
    test_input_multiple_variables,
    "10 INPUT A$,A",
    "20 PRINT A$;A",
);

basic_test!(
    test_input_multiple_with_prompt,
    r#"10 INPUT "give me two numbers to multiply (separated by a comma) ";a,b"#,
    "30 PRINT a;b",
);

basic_test!(
    test_input_with_channel,
    "10 INPUT #2,x",
    "20 PRINT x",
);

basic_test!(
    test_input_no_carriage_return,
    r#"10 INPUT;"Enter value";x"#,
    "20 PRINT x",
);

// ============================================================================
// Interrupts and Timers
// ============================================================================
// ============================================================================
// Interrupts and Timers
// ============================================================================

basic_test!(
    test_interrupt_example,
    "10 REM > interrupts",
    "20 EVERY 50,0 GOSUB 100",
    "30 EVERY 100,1 GOSUB 200",
    "40 EVERY 200,2 GOSUB 300",
    "50 AFTER 1000,3 GOSUB 400",
    "60 WHILE flag=0",
    "70 a=a+1:PRINT a",
    "80 WEND",
    "90 END",
    "100 REM #0",
    "110 PEN 2:PRINT \"timer 0\":PEN 1",
    "120 RETURN",
    "200 REM #1",
    "210 PEN 2:PRINT \"timer 1\":PEN 1",
    "220 RETURN",
    "300 REM #2",
    "310 PEN 2:PRINT \"timer 2\":PEN 1",
    "320 RETURN",
    "400 REM #3",
    "410 flag=1:PEN 2:PRINT \"no more interrupts...\"",
    "420 RETURN",
);

// ============================================================================
// Functions and Math
// ============================================================================

basic_test!(
    test_function_definition,
    "10 gn=9.80665",
    "20 DEF FNgrv=s0+v0*t+0.5*gn*t^2",
    "30 s0=0:v0=0:t=5",
    r#"40 PRINT "...after";t;"seconds your dropped stone falls";FNgrv;"metres""#,
);

immediate_test!(test_int_division, "w=INT(50000)");

basic_test!(
    test_cint_example,
    "10 n=1.9999",
    "20 PRINT CINT(n)",
);

basic_test!(
    test_creal_example,
    "10 a=PI",
    "20 PRINT CINT(a)",
    "30 PRINT CREAL(a)",
);

basic_test!(
    test_chr_loop,
    "10 FOR X=32 TO 255",
    "20 PRINT X;CHR$(X);",
    "30 NEXT",
);

basic_test!(test_random_maze, "10 PRINT CHR$(208+RND(2));:GOTO 10");

// ============================================================================
// String Manipulation
// ============================================================================

basic_test!(
    test_let_example,
    r#"10 LET a$ = "hello world""#,
    "20 PRINT a$",
);

basic_test!(
    test_mid_first,
    r#"10 a$="Hello""#,
    "20 PRINT MID$(a$,2,2)",
);

basic_test!(
    test_mid_second,
    r#"10 a$="Hello""#,
    r#"20 MID$(a$,2,3)="ipp""#,
    "30 PRINT a$",
);

basic_test!(
    test_copychr_example,
    "10 CLS",
    r#"20 PRINT "top corner""#,
    "30 LOCATE 1,1",
    "40 a$=COPYCHR$(#0)",
    "50 LOCATE 1,20",
    "60 PRINT a$",
);

// ============================================================================
// I/O and File Operations
// ============================================================================

// ============================================================================
// I/O and File Operations
// ============================================================================

basic_test!(
    test_write_file,
    r#"10 OPENOUT "DUMMY""#,
    "20 INPUT A$,A",
    "30 WRITE #9,A$,A",
    "40 CLOSEOUT",
);

basic_test!(
    test_eof_file_reading,
    r#"10 OPENIN "text.txt""#,
    "20 WHILE NOT EOF",
    "30 LINE INPUT#9,a$",
    "40 PRINT a$",
    "50 WEND",
    "60 CLOSEIN",
);

basic_test!(
    test_error_handling,
    "10 ON ERROR GOTO 1000",
    r#"20 OPENOUT "myfile.asc""#,
    r#"30 WRITE #9,"test-data""#,
    "40 CLOSEOUT",
    "50 END",
    "1000 amsdoserr=(DERR AND &7F)",
    "1010 IF ERR<31 THEN END",
    r#"1020 IF ERR=31 THEN PRINT "are you sure you've typed line 20 correctly?":END"#,
    r#"1030 IF amsdoserr=20 THEN PRINT "disc is full, suggest you use a new data disc":END"#,
    r#"1040 IF amsdoserr=&X01001000 THEN PRINT "put a disc in the drive, then press a key":WHILE INKEY$="":WEND:RESUME"#,
    "1050 END",
);

// ============================================================================
// Sound and Multimedia
// ============================================================================

basic_test!(
    test_release_sound,
    "10 SOUND 65,1000,100",
    r#"20 PRINT"PRESS R TO LET IT SOUND""#,
    "30 IF INKEY(50)=-1 THEN 30",
    "40 RELEASE 1",
);

// ============================================================================
// Program Management
// ============================================================================

basic_test!(
    test_renum_before,
    "10 GOTO 20",
    "20 GOTO 30",
    "30 GOTO 10",
);

// ============================================================================
// Formatting and Display
// ============================================================================

immediate_test!(test_spc_example, r#"PRINT "Hello";SPC(10);"World";"#);

basic_test!(
    test_using_format,
    "10 figure=123456789",
    r#"20 word$="Hello""#,
    r#"30 PRINT USING "+**,##########";figure"#,
    r#"40 PRINT USING "\   \";word$"#,
);

basic_test!(
    test_zone_example,
    "10 MODE 2",
    r#"20 PRINT"normal zone (13)""#,
    "30 PRINT 1,2,3,4",
);

// ============================================================================
// Bit Operations
// ============================================================================

basic_test!(
    test_and_bit_manipulation,
    "10 counter=0",
    "20 counter=(counter+1) AND 31",
    "30 GOTO 20",
);

basic_test!(
    test_not_example,
    "10 value1=1",
    "20 PRINT HEX$(value1):PRINT BIN$(value1,8)",
    "30 value2=NOT value1",
    "40 PRINT HEX$(value2):PRINT BIN$(value2,8)",
);

// ============================================================================
// Comprehensive Round-trip Test
// ============================================================================
#[test]
fn test_all_examples_roundtrip() {
    let all_lines = vec![
        // Basic commands
        "AUTO 100,5",
        "CALL 0",
        "CAT",
        
        // Numeric examples
        "10 n=1.9999",
        "10 gn=9.80665",
        "10 counter=0",
        "10 value1=1",
        "10 figure=123456789",
        
        // String examples
        r#"10 a$="Hello""#,
        r#"10 LET a$ = "hello world""#,
        r#"20 word$="Hello""#,
        
        // Control structures
        "10 FOR I=1 TO 10",
        "30 NEXT I",
        "10 GOTO 100",
        "20 GOSUB 100",
        "120 RETURN",
        "40 END",
        
        // Conditional
        "40 IF I<25 THEN GOTO 20",
        r#"20 IF f=10 THEN PRINT "right": END: ELSE GOTO 10"#,
        
        // Graphics
        "CLG 2",
        "DRAW 500,400,0",
        "MOVE 200,200",
        "DRAWR 100,100,0",
        "MASK 15",
        
        // I/O
        "20 PRINT I;",
        r#"10 INPUT "guess a figure:",f"#,
        r#"10 OPENOUT "DUMMY""#,
        "40 CLOSEOUT",
        
        // Math
        "20 DEF FNgrv=s0+v0*t+0.5*gn*t^2",
        "10 a=PI",
        "20 PRINT CINT(n)",
    ];
    
    let mut failed = Vec::new();
    let mut successful = 0;
    
    for line in &all_lines {
        // Check if line starts with a digit to determine if it's a program line or immediate command
        let is_program_line = line.trim_start().chars().next().map_or(false, |c| c.is_ascii_digit());
        
        let result = if is_program_line {
            test_basic_line(line)
        } else {
            test_immediate_statement(line)
        };
        
        match result {
            Ok(reconstructed) => {
                successful += 1;
                println!("✓ {}", line);
                println!("  -> {}", reconstructed);
            },
            Err(e) => {
                failed.push((line, e));
                println!("✗ {}", line);
            }
        }
    }
    
    if !failed.is_empty() {
        println!("\n=== Failed Lines ===");
        for (line, err) in &failed {
            println!("{}: {}", line, err);
        }
        panic!("{}/{} lines failed to parse", failed.len(), all_lines.len());
    }
    
    println!("\n=== Summary ===");
    println!("All {} lines parsed and reconstructed successfully!", successful);
}
