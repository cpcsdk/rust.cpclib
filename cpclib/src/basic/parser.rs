use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::sequence::*;
///! Locomotive basic parser routines.
use nom::*;

use crate::basic::tokens::*;
use crate::basic::{BasicLine, BasicProgram};

/// Parse complete basic program"],
pub fn parse_basic_program(input: &str) -> IResult<&str, BasicProgram> {
    let (input, lines) = fold_many0(
        parse_basic_inner_line,
        Vec::new(),
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        },
    )(input)?;

    let (input, last) = terminated(opt(parse_basic_line), opt(line_ending))(input)?;

    let mut lines = lines.clone();
    if let Some(line) = last {
        lines.push(line);
    }

    Ok((input, BasicProgram::new(lines)))
}

/// Parse a line
pub fn parse_basic_line(input: &str) -> IResult<&str, BasicLine> {
    let (input, line_number) = dec_u16_inner(input)?;

    let (input, _) = char(' ')(input)?;

    let (input, tokens) = fold_many0(parse_token, Vec::new(), |mut acc: Vec<_>, item| {
        acc.push(item);
        acc
    })(input)?;

    Ok((input, BasicLine::new(line_number, &tokens)))
}

/// Parse a line BUT expect an end of line char
pub fn parse_basic_inner_line(input: &str) -> IResult<&str, BasicLine> {
    terminated(parse_basic_line, line_ending)(input)
}

/// Parse any token
pub fn parse_token(input: &str) -> IResult<&str, BasicToken> {
    alt((
        parse_rem,
        parse_simple_instruction,
        parse_prefixed_instruction,
        parse_basic_value,
        parse_char,
    ))(input)
}

/// Parse a comment"],
pub fn parse_rem(input: &str) -> IResult<&str, BasicToken> {
    let (input, sym) = alt((
        map(tag_no_case("REM"), { |_| BasicTokenNoPrefix::Rem }),
        map(char('\''), { |_| BasicTokenNoPrefix::SymbolQuote }),
    ))(input)?;

    let (input, list) = take_till(|ch| ch == ':' || ch == '\n')(input)?;

    Ok((input, BasicToken::Comment(sym, list.as_bytes().to_vec())))
}

/// Parse the instructions that do not need a prefix byte
/// TODO Add all the other variants"
pub fn parse_simple_instruction(input: &str) -> IResult<&str, BasicToken> {
    map(
        alt((
            map(tag_no_case("CALL"), { |_| BasicTokenNoPrefix::Call }),
            map(tag_no_case("INPUT"), { |_| BasicTokenNoPrefix::Input }),
            map(tag_no_case("PRINT"), { |_| BasicTokenNoPrefix::Print }),
        )),
        |token| BasicToken::SimpleToken(token),
    )(input)
}

/// TODO add the missing chars
pub fn parse_char(input: &str) -> IResult<&str, BasicToken> {
    map(
        alt((
            alt((
                map(char(':'), { |_| BasicTokenNoPrefix::StatementSeparator }),
                map(char(' '), { |_| BasicTokenNoPrefix::CharSpace }),
                map(char('A'), { |_| BasicTokenNoPrefix::CharUpperA }),
                map(char('B'), { |_| BasicTokenNoPrefix::CharUpperB }),
                map(char('C'), { |_| BasicTokenNoPrefix::CharUpperC }),
                map(char('D'), { |_| BasicTokenNoPrefix::CharUpperD }),
                map(char('E'), { |_| BasicTokenNoPrefix::CharUpperE }),
                map(char('F'), { |_| BasicTokenNoPrefix::CharUpperF }),
                map(char('G'), { |_| BasicTokenNoPrefix::CharUpperG }),
                map(char('H'), { |_| BasicTokenNoPrefix::CharUpperH }),
                map(char('I'), { |_| BasicTokenNoPrefix::CharUpperI }),
                map(char('J'), { |_| BasicTokenNoPrefix::CharUpperJ }),
                map(char('K'), { |_| BasicTokenNoPrefix::CharUpperK }),
                map(char('L'), { |_| BasicTokenNoPrefix::CharUpperL }),
                map(char('M'), { |_| BasicTokenNoPrefix::CharUpperM }),
                map(char('N'), { |_| BasicTokenNoPrefix::CharUpperN }),
                map(char('O'), { |_| BasicTokenNoPrefix::CharUpperO }),
                map(char('P'), { |_| BasicTokenNoPrefix::CharUpperP }),
                map(char('Q'), { |_| BasicTokenNoPrefix::CharUpperQ }),
                map(char('R'), { |_| BasicTokenNoPrefix::CharUpperR }),
            )),
            alt((
                map(char('S'), { |_| BasicTokenNoPrefix::CharUpperS }),
                map(char('T'), { |_| BasicTokenNoPrefix::CharUpperT }),
                map(char('U'), { |_| BasicTokenNoPrefix::CharUpperU }),
                map(char('V'), { |_| BasicTokenNoPrefix::CharUpperV }),
                map(char('W'), { |_| BasicTokenNoPrefix::CharUpperW }),
                map(char('X'), { |_| BasicTokenNoPrefix::CharUpperX }),
                map(char('Y'), { |_| BasicTokenNoPrefix::CharUpperY }),
                map(char('Z'), { |_| BasicTokenNoPrefix::CharUpperZ }),
            )),
            alt((
                map(char('a'), { |_| BasicTokenNoPrefix::CharLowerA }),
                map(char('b'), { |_| BasicTokenNoPrefix::CharLowerB }),
                map(char('c'), { |_| BasicTokenNoPrefix::CharLowerC }),
                map(char('d'), { |_| BasicTokenNoPrefix::CharLowerD }),
                map(char('e'), { |_| BasicTokenNoPrefix::CharLowerE }),
                map(char('f'), { |_| BasicTokenNoPrefix::CharLowerF }),
                map(char('g'), { |_| BasicTokenNoPrefix::CharLowerG }),
                map(char('h'), { |_| BasicTokenNoPrefix::CharLowerH }),
                map(char('i'), { |_| BasicTokenNoPrefix::CharLowerI }),
                map(char('j'), { |_| BasicTokenNoPrefix::CharLowerJ }),
                map(char('k'), { |_| BasicTokenNoPrefix::CharLowerK }),
                map(char('l'), { |_| BasicTokenNoPrefix::CharLowerL }),
                map(char('m'), { |_| BasicTokenNoPrefix::CharLowerM }),
                map(char('n'), { |_| BasicTokenNoPrefix::CharLowerN }),
                map(char('o'), { |_| BasicTokenNoPrefix::CharLowerO }),
            )),
            alt((
                map(char('p'), { |_| BasicTokenNoPrefix::CharLowerP }),
                map(char('q'), { |_| BasicTokenNoPrefix::CharLowerQ }),
                map(char('r'), { |_| BasicTokenNoPrefix::CharLowerR }),
                map(char('s'), { |_| BasicTokenNoPrefix::CharLowerS }),
                map(char('t'), { |_| BasicTokenNoPrefix::CharLowerT }),
                map(char('u'), { |_| BasicTokenNoPrefix::CharLowerU }),
                map(char('v'), { |_| BasicTokenNoPrefix::CharLowerV }),
                map(char('w'), { |_| BasicTokenNoPrefix::CharLowerW }),
                map(char('x'), { |_| BasicTokenNoPrefix::CharLowerX }),
                map(char('y'), { |_| BasicTokenNoPrefix::CharLowerY }),
                map(char('z'), { |_| BasicTokenNoPrefix::CharLowerZ }),
            )),
        )),
        |token| BasicToken::SimpleToken(token),
    )(input)
}

/// Parse the instructions that do not need a prefix byte
/// TODO Add all the other instructions"],
pub fn parse_prefixed_instruction(input: &str) -> IResult<&str, BasicToken> {
    map(
        alt((
            value(BasicTokenPrefixed::Abs, tag_no_case("ABS")),
            value(BasicTokenPrefixed::Abs, tag_no_case("ABS")), //TODO put the others
        )),
        |token| BasicToken::PrefixedToken(token),
    )(input)
}

/// Parse a basic value
pub fn parse_basic_value(input: &str) -> IResult<&str, BasicToken> {
    alt((parse_hexadecimal_value_16bits, parse_decimal_value_16bits))(input)
}

/// Parse an hexadecimal value
pub fn parse_hexadecimal_value_16bits(input: &str) -> IResult<&str, BasicToken> {
    map(preceded(char('&'), hex_u16_inner), |val| {
        BasicToken::Constant(
            BasicTokenNoPrefix::ValueIntegerHexadecimal16bits,
            BasicValue::new_integer(val),
        )
    })(input)
}

/// ...
pub fn parse_decimal_value_16bits(input: &str) -> IResult<&str, BasicToken> {
    map(dec_u16_inner, |val| {
        BasicToken::Constant(
            BasicTokenNoPrefix::ValueIntegerDecimal16bits,
            BasicValue::new_integer(val),
        )
    })(input)
}

/// XXX stolen to the asm parser
#[inline]
pub fn hex_u16_inner(input: &str) -> IResult<&str, u16> {
    match is_a("0123456789abcdefABCDEF")(input) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than  characters for a u16
            if parsed.input_len() > 4 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0_u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(16).unwrap_or(0);
                    res = value + (res * 16);
                }
                if res > u32::from(u16::max_value()) {
                    Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

/// XXX stolen to the asm parser
#[inline]
pub fn dec_u16_inner(input: &str) -> IResult<&str, u16> {
    match is_a("0123456789")(input) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than 5 characters for a u16
            if parsed.input_len() > 5 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0_u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(10).unwrap_or(0);
                    res = value + (res * 10);
                }
                if res > u32::from(u16::max_value()) {
                    Err(::nom::Err::Error(error_position!(
                        input,
                        ErrorKind::TooLarge /*Custom(0)*/
                    )))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::basic::parser::*;

    #[test]
    fn check_number() {
        assert!(dec_u16_inner("10").is_ok());

        match hex_u16_inner("1234".into()) {
            Ok((res, value)) => {
                println!("{:?}", &res);
                println!("{:x}", &value);
                assert_eq!(0x1234, value);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }

        match parse_hexadecimal_value_16bits("&1234".into()) {
            Ok((res, value)) => {
                println!("{:?}", &res);
                println!("{:?}", &value);
                let bytes = value.as_bytes();
                assert_eq!(
                    bytes[0],
                    BasicTokenNoPrefix::ValueIntegerHexadecimal16bits as u8
                );
                assert_eq!(bytes[1], 0x34);
                assert_eq!(bytes[2], 0x12);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_line_tokenisation(code: &str) -> BasicLine {
        let res = parse_basic_inner_line(code.into());
        match res {
            Ok((res, line)) => {
                println!("{:?}", &line);
                println!("{:?}", &res);
                assert_eq!(res.len(), 0, "Line as not been completly consummed");
                line
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn check_token_tokenisation(code: &str) {
        let res = parse_token(code.into());
        match res {
            Ok((res, line)) => {
                println!("{} => {:?}", code, &line);
                assert_eq!(res.len(), 0, "Line as not been completly consummed");
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn test_lines() {
        check_line_tokenisation("10 call &0\n");
        check_line_tokenisation("10 call &0  \n");
        check_line_tokenisation("10 call &0: call &0\n");
    }

    #[test]
    fn test_tokens() {
        check_token_tokenisation("call");
        check_token_tokenisation("abs");
        check_token_tokenisation(":");
    }

    #[test]
    fn test_comment() {
        check_token_tokenisation("REM fldsfksjfksjkg");
        check_token_tokenisation("' fldsfksjfksjkg");

        let line = check_line_tokenisation("10 REM fldsfksjfksjkg:CALL\n");
        assert_eq!(3, line.tokens().len())
    }
}
