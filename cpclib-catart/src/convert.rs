use cpclib_basic::tokens::{BasicToken, BasicTokenNoPrefix, BasicTokenPrefixed, BasicValue};
use cpclib_basic::{BasicLine, BasicProgram};

use crate::basic_command::{BasicCommand, BasicCommandList, PrintArgument, PrintTerminator};
use crate::error::CatArtError;

fn consume_integer_argument(iter: &mut Vec<(u16, &BasicToken)>) -> Result<u8, CatArtError> {
    let mut pair = None;

    // consume space
    let mut eat_space = true;
    while eat_space {
        let res = iter.pop().ok_or(CatArtError::NotEnoughTokens(
            "Unexpected end of tokens".to_string()
        ))?;
        pair = Some(res);
        match res.1 {
            BasicToken::SimpleToken(BasicTokenNoPrefix::CharSpace) => {
                // continue eating spaces
            },
            _ => {
                eat_space = false;
            }
        }
    }

    if let Some((line, curr)) = pair {
        // curr is the current non-space token
        // we expect it to be an integer value
        match curr {
            BasicToken::Constant(_, value) => {
                match value.as_integer() {
                    Some(val) => Ok(val as _),
                    None => {
                        Err(CatArtError::InvalidParameter(
                            line,
                            "An integer value is expected".to_string()
                        ))
                    },
                }
            },
            _ => {
                Err(CatArtError::IncompatibleBasicCommand(
                    line,
                    "Expected integer argument".to_string()
                ))
            }
        }
    }
    else {
        Err(CatArtError::NotEnoughTokens(
            "Unexpected end of tokens".to_string()
        ))
    }
}

/// Consume a comma (with optional spaces before and after)
fn consume_comma(iter: &mut Vec<(u16, &BasicToken)>) -> Result<(), CatArtError> {
    // consume optional spaces before comma
    loop {
        let res = iter
            .pop()
            .ok_or(CatArtError::NotEnoughTokens("Expected comma".to_string()))?;
        match res.1 {
            BasicToken::SimpleToken(BasicTokenNoPrefix::CharSpace) => {
                // continue eating spaces
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::CharComma) => {
                // found comma, consume optional spaces after
                while let Some((_, BasicToken::SimpleToken(BasicTokenNoPrefix::CharSpace))) =
                    iter.last()
                {
                    iter.pop();
                }
                return Ok(());
            },
            _ => {
                return Err(CatArtError::IncompatibleBasicCommand(
                    res.0,
                    "Expected comma".to_string()
                ));
            }
        }
    }
}

/// Consume INK arguments: pen (0-15), ink1 (0-31), and optional ink2 (0-31)
/// If ink2 is not provided, it defaults to ink1
fn consume_ink_arguments(
    iter: &mut Vec<(u16, &BasicToken)>
) -> Result<(u8, u8, Option<u8>), CatArtError> {
    let pen = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let ink1 = consume_integer_argument(iter)?;

    // Check if there's a comma ahead (peek before consuming)
    consume_spaces(iter);
    let has_third_arg = matches!(
        iter.last(),
        Some((_, BasicToken::SimpleToken(BasicTokenNoPrefix::CharComma)))
    );

    let ink2 = if has_third_arg {
        // Third argument present, consume it
        consume_comma(iter)?;
        Some(consume_integer_argument(iter)?)
    }
    else {
        // No third argument, use ink1 as ink2
        None
    };

    Ok((pen, ink1, ink2))
}

/// Consume LOCATE arguments: column and row
fn consume_locate_arguments(
    iter: &mut Vec<(u16, &BasicToken)>
) -> Result<(u8, u8), CatArtError> {
    let col = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let row = consume_integer_argument(iter)?;
    Ok((col, row))
}

/// Consume WINDOW arguments: left, right, top, bottom
fn consume_window_arguments(
    iter: &mut Vec<(u16, &BasicToken)>
) -> Result<(u8, u8, u8, u8), CatArtError> {
    let left = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let right = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let top = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let bottom = consume_integer_argument(iter)?;
    Ok((left, right, top, bottom))
}

/// Consume SYMBOL arguments: character code followed by 8 row bytes
/// A `SYMBOL` command's arguments: character code followed by its 8 row bytes.
type SymbolArguments = (u8, [u8; 8]);

fn consume_symbol_arguments(
    iter: &mut Vec<(u16, &BasicToken)>
) -> Result<SymbolArguments, CatArtError> {
    let char_code = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r1 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r2 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r3 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r4 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r5 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r6 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r7 = consume_integer_argument(iter)?;
    consume_comma(iter)?;
    let r8 = consume_integer_argument(iter)?;
    Ok((char_code, [r1, r2, r3, r4, r5, r6, r7, r8]))
}

/// Consume optional spaces
fn consume_spaces(iter: &mut Vec<(u16, &BasicToken)>) {
    while let Some((_, token)) = iter.last() {
        match token {
            BasicToken::SimpleToken(BasicTokenNoPrefix::CharSpace) => {
                iter.pop();
            },
            _ => break
        }
    }
}

/// Consume all PRINT items and track whether there's a trailing semicolon
/// Returns (collected bytes, has_trailing_semicolon)
fn consume_print_statement(
    iter: &mut Vec<(u16, &BasicToken)>
) -> Result<(PrintArgument, PrintTerminator), CatArtError> {
    let mut args = Vec::new();
    let mut has_trailing_semicolon = false;

    // Check if empty print (just PRINT)
    consume_spaces(iter);
    if let Some((
        _,
        BasicToken::SimpleToken(BasicTokenNoPrefix::EndOfTokenisedLine)
        | BasicToken::SimpleToken(BasicTokenNoPrefix::StatementSeparator)
    )) = iter.last()
    {
        return Ok((PrintArgument::Composite(args), PrintTerminator::CrLf));
    }

    loop {
        consume_spaces(iter);

        // Check what's next
        let pair = match iter.last() {
            Some(p) => *p,
            None => break // End of program
        };

        match pair.1 {
            // String literal - CommentOrString token with bytes
            BasicToken::CommentOrString(BasicTokenNoPrefix::ValueQuotedString, bytes, _) => {
                iter.pop(); // consume the token
                args.push(PrintArgument::String(bytes.clone()));
                has_trailing_semicolon = false; // Reset - only true if semicolon comes after content
            },
            // String literal - match any other Constant with String value (fallback)
            BasicToken::Constant(_, BasicValue::String(s)) => {
                iter.pop(); // consume the token
                args.push(PrintArgument::String(s.as_bytes().to_vec()));
                has_trailing_semicolon = false; // Reset - only true if semicolon comes after content
            },
            // CHR$ function
            BasicToken::PrefixedToken(BasicTokenPrefixed::ChrDollar) => {
                iter.pop(); // consume CHR$

                // Expect opening parenthesis
                consume_spaces(iter);
                let next = iter.pop().ok_or(CatArtError::NotEnoughTokens(
                    "Expected ( after CHR$".to_string()
                ))?;
                if !matches!(
                    next.1,
                    BasicToken::SimpleToken(BasicTokenNoPrefix::CharOpenParenthesis)
                ) {
                    return Err(CatArtError::IncompatibleBasicCommand(
                        next.0,
                        "Expected ( after CHR$".to_string()
                    ));
                }

                // Get the character code
                let char_code = consume_integer_argument(iter)?;

                // Expect closing parenthesis
                consume_spaces(iter);
                let next = iter.pop().ok_or(CatArtError::NotEnoughTokens(
                    "Expected ) after CHR$ argument".to_string()
                ))?;
                if !matches!(
                    next.1,
                    BasicToken::SimpleToken(BasicTokenNoPrefix::CharCloseParenthesis)
                ) {
                    return Err(CatArtError::IncompatibleBasicCommand(
                        next.0,
                        "Expected ) after CHR$ argument".to_string()
                    ));
                }

                args.push(PrintArgument::ChrDollar(char_code));
                has_trailing_semicolon = false; // Reset
            },
            // Semicolon - continue collecting
            BasicToken::SimpleToken(BasicTokenNoPrefix::CharSemiColon) => {
                iter.pop();
                has_trailing_semicolon = true; // Mark that we have a semicolon
                // Continue to check if there's more content
            },
            // End of statement - stop
            BasicToken::SimpleToken(BasicTokenNoPrefix::EndOfTokenisedLine)
            | BasicToken::SimpleToken(BasicTokenNoPrefix::StatementSeparator) => {
                break;
            },
            _ => {
                // Unknown token in PRINT context - debug and stop
                // Unknown token in PRINT context: ignored
                break;
            }
        }
    }

    let arg = if args.len() == 1 {
        args.pop().unwrap()
    }
    else {
        PrintArgument::Composite(args)
    };

    let terminator = if has_trailing_semicolon {
        PrintTerminator::None
    }
    else {
        PrintTerminator::CrLf
    };

    Ok((arg, terminator))
}

/// Convert a BASIC program to its control character equivalent.
pub fn basic_to_commands<'b>(basic: &'b BasicProgram) -> Result<BasicCommandList, CatArtError> {
    // get the tokens in reverse order to pop them
    let mut line_token_pairs: Vec<(u16, &'b BasicToken)> = basic
        .lines()
        .iter()
        .flat_map(|line: &BasicLine| {
            let nb = line.line_number();
            line.tokens().iter().map(move |t| (nb, t))
        })
        .rev()
        .collect::<Vec<_>>();

    let mut commands: Vec<BasicCommand> = Vec::new();

    while let Some((line, token)) = line_token_pairs.pop() {
        match token {
            BasicToken::CommentOrString(..) => {}, // we ignore comments
            BasicToken::SimpleToken(BasicTokenNoPrefix::Paper) => {
                let value = consume_integer_argument(&mut line_token_pairs)?;
                commands.push(BasicCommand::paper(value));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Pen) => {
                let value = consume_integer_argument(&mut line_token_pairs)?;
                commands.push(BasicCommand::pen(value));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Mode) => {
                let value = consume_integer_argument(&mut line_token_pairs)?;
                commands.push(BasicCommand::mode(value));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Ink) => {
                let (pen, ink1, ink2) = consume_ink_arguments(&mut line_token_pairs)?;
                commands.push(BasicCommand::ink(pen, ink1, ink2));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Border) => {
                // BORDER ink1 [, ink2]
                let ink1 = consume_integer_argument(&mut line_token_pairs)?;
                // Check for optional comma and ink2
                consume_spaces(&mut line_token_pairs);
                let ink2 =
                    if let Some((_, BasicToken::SimpleToken(BasicTokenNoPrefix::CharComma))) =
                        line_token_pairs.last()
                    {
                        line_token_pairs.pop();
                        Some(consume_integer_argument(&mut line_token_pairs)?)
                    }
                    else {
                        None
                    };
                commands.push(BasicCommand::border(ink1, ink2));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Locate) => {
                let (col, row) = consume_locate_arguments(&mut line_token_pairs)?;
                commands.push(BasicCommand::locate(col, row));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Window) => {
                let (left, right, top, bottom) = consume_window_arguments(&mut line_token_pairs)?;
                commands.push(BasicCommand::window(left, right, top, bottom));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Print) => {
                let (arg, terminator) = consume_print_statement(&mut line_token_pairs)?;
                commands.push(BasicCommand::PrintString(arg, terminator));
            },
            BasicToken::SimpleToken(BasicTokenNoPrefix::Cls) => {
                commands.push(BasicCommand::cls());
            },
            // CURSOR n[,n] - real CPC firmware's system/user cursor flags.
            // Structurally valid, but a no-op in CatArt's own renderer (it
            // corresponds to printed CHR$(2)/CHR$(3) control codes, which
            // CatArt ignores) - the caller decides whether to warn about it.
            BasicToken::SimpleToken(BasicTokenNoPrefix::Cursor) => {
                let first = consume_integer_argument(&mut line_token_pairs)?;
                // Optional second argument (CURSOR n,n form) - only the
                // first arg determines on/off; consume and discard the rest.
                consume_spaces(&mut line_token_pairs);
                if let Some((_, BasicToken::SimpleToken(BasicTokenNoPrefix::CharComma))) =
                    line_token_pairs.last()
                {
                    line_token_pairs.pop();
                    consume_integer_argument(&mut line_token_pairs)?;
                }
                commands.push(if first == 0 {
                    BasicCommand::cursor_off()
                }
                else {
                    BasicCommand::cursor_on()
                });
            },
            // SYMBOL char,row1..row8 - structurally valid, but a no-op in
            // CatArt's own renderer (corresponds to printed CHR$(25) + 9
            // bytes, which CatArt ignores) - the caller decides whether to
            // warn about it.
            BasicToken::SimpleToken(BasicTokenNoPrefix::Symbol) => {
                let (char_code, rows) = consume_symbol_arguments(&mut line_token_pairs)?;
                commands.push(BasicCommand::symbol(char_code, rows));
            },
            // Skip statement separators, end-of-line markers, spaces, and colon (:) separators
            BasicToken::SimpleToken(BasicTokenNoPrefix::EndOfTokenisedLine)
            | BasicToken::SimpleToken(BasicTokenNoPrefix::StatementSeparator)
            | BasicToken::SimpleToken(BasicTokenNoPrefix::CharSpace)
            | BasicToken::SimpleToken(BasicTokenNoPrefix::CharColon) => {},
            _ => {
                return Err(CatArtError::IncompatibleBasicCommand(
                    line,
                    format!("Unsupported BASIC command token: {:?}", token)
                ));
                // Token ignored: not a supported command
            }
        }
    }

    Ok(commands.into())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_paper() {
        let code = r###"10 PAPER 2"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::paper(2)].into());
    }

    #[test]
    fn test_pen() {
        let code = r###"10 PEN 1"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::pen(1)].into());
    }

    #[test]
    fn test_mode() {
        let code = r###"10 MODE 0"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::mode(0)].into());
    }

    #[test]
    fn test_ink() {
        let code = r###"10 INK 2, 4, 5"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::ink(2, 4, Some(5))].into());

        let code = r###"10 INK 2, 3"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::ink(2, 3, None)].into());
    }

    #[test]
    fn test_print() {
        let code = r###"10 PRINT "HELLO""###;
        let basic = dbg!(BasicProgram::parse(code)).expect("Tokenization failed");
        let result = dbg!(basic_to_commands(&basic)).expect("Conversion failed");
        assert_eq!(
            result,
            vec![BasicCommand::print_string_crlf(br#"HELLO"#.to_vec())].into()
        );

        let code = r###"10 PRINT "HELLO";"###;
        let basic = dbg!(BasicProgram::parse(code)).expect("Tokenization failed");
        let result = dbg!(basic_to_commands(&basic)).expect("Conversion failed");
        assert_eq!(
            result,
            vec![BasicCommand::print_string(br#"HELLO"#.to_vec())].into()
        );

        let code = r###"10 PRINT"HELLO""###;
        let basic = dbg!(BasicProgram::parse(code)).expect("Tokenization failed");
        let result = dbg!(basic_to_commands(&basic)).expect("Conversion failed");
        assert_eq!(
            result,
            vec![BasicCommand::print_string_crlf(br#"HELLO"#.to_vec())].into()
        );
    }

    #[test]
    fn test_cursor_off() {
        let code = r###"10 CURSOR 0"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::cursor_off()].into());
    }

    #[test]
    fn test_cursor_on() {
        let code = r###"10 CURSOR 1"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::cursor_on()].into());
    }

    #[test]
    fn test_cursor_on_with_second_argument() {
        let code = r###"10 CURSOR 1,1"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(result, vec![BasicCommand::cursor_on()].into());
    }

    #[test]
    fn test_symbol() {
        let code = r###"10 SYMBOL 65,1,2,3,4,5,6,7,8"###;
        let basic = BasicProgram::parse(code).expect("Tokenization failed");
        let result = basic_to_commands(&basic).expect("Conversion failed");
        assert_eq!(
            result,
            vec![BasicCommand::symbol(65, [1, 2, 3, 4, 5, 6, 7, 8])].into()
        );
    }
}
