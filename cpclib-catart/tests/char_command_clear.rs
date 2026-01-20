// Tests for CharCommand generation from CHR$(18) and CHR$(20)
// and for correct screen effects of ClearLineEnd and ClearScreenEnd

use cpclib_catart::char_command::CharCommand;
use cpclib_catart::interpret::{Interpreter, Mode};

#[test]
fn test_char_command_from_chr18_and_chr20() {
    // CHR$(18) should map to ClearLineEnd
    let cc18 = CharCommand::char_to_command_or_count(18u8).unwrap();
    assert_eq!(cc18, CharCommand::ClearLineEnd);
    // CHR$(20) should map to ClearScreenEnd
    let cc20 = CharCommand::char_to_command_or_count(20u8).unwrap();
    assert_eq!(cc20, CharCommand::ClearScreenEnd);
}

#[test]
fn test_interpreter_clear_line_end() {
    let mut interp = Interpreter::new(Mode::Mode1);
    // Write some text, then ClearLineEnd
    interp.interpret(&[
        CharCommand::Locate(1, 1),
        CharCommand::PrintSymbol(b'A'),
        CharCommand::PrintSymbol(b'B'),
        CharCommand::PrintSymbol(b'C'),
        CharCommand::Locate(1, 1),
        CharCommand::ClearLineEnd,
    ], false).unwrap();
    // The rest of the line after the cursor should be filled with paper color
    let screen = interp.screen();
    dbg!(screen);
    let (width, _) = screen.resolution();
    for x in 1..=width {
        dbg!(x);
        let cell = screen.cell(x, 1).unwrap();
        assert_eq!(cell.ch, b' ');
    }
}

#[test]
fn test_interpreter_clear_screen_end() {
    let mut interp = Interpreter::new(Mode::Mode1);
    // Write some text, then ClearScreenEnd
    interp.interpret(&[
        CharCommand::PrintSymbol(b'A'),
        CharCommand::PrintSymbol(b'B'),
        CharCommand::CursorDown,
        CharCommand::PrintSymbol(b'C'),
        CharCommand::ClearScreenEnd,
    ], false).unwrap();
    // The rest of the current line and all lines below should be filled with paper color
    let screen = interp.screen();
    let (width, height) = screen.resolution();
    // Current line after cursor
    for x in 4..=width {
        let cell = screen.cell(x, 2).unwrap();
        assert_eq!(cell.ch, b' ');
    }
    // All lines below
    for y in 3..=height {
        for x in 1..=width {
            let cell = screen.cell(x, y).unwrap();
            assert_eq!(cell.ch, b' ');
        }
    }
}
