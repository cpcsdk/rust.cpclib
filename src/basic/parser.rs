///! Locomotive basic parser routines.
use nom::named_attr;
use nom::types::CompleteStr;
use nom::InputIter;
use nom::InputLength;
use nom::*;
use nom::{line_ending, ErrorKind, IResult};

use crate::basic::tokens::*;
use crate::basic::{BasicLine, BasicProgram};

named_attr!(#[doc=" Parse complete basic program"],
	pub parse_basic_program<CompleteStr<'_>, BasicProgram>, do_parse!(
		lines: fold_many0!(
			parse_basic_inner_line,
			Vec::new(),
			|mut acc:Vec<_>, item|{
				acc.push(item);
				acc
			}
		) >>
		last: opt!(parse_basic_line) >>
		opt!(line_ending) >>
		( {
			let mut lines = lines.clone();
			if let Some(line) = last {
				lines.push(line);
			}
			BasicProgram::new(lines)
		  }
		)
	)
);

named_attr!(#[doc=" Parse a line"],
	pub parse_basic_line<CompleteStr<'_>, BasicLine>, do_parse!(
		line_number: dec_u16_inner >>
		char!(' ') >> 
		tokens: fold_many0!(
			parse_token,
			Vec::new(),
			|mut acc:Vec<_>, item|{
				acc.push(item);
				acc
			}
		) >>
		(
			BasicLine::new(
				line_number,
				&tokens
			)
		)
	)
);

named_attr!(#[doc=" Parse a line BUT expect an end of line char"],
	pub parse_basic_inner_line<CompleteStr<'_>, BasicLine>, do_parse!(
		line: parse_basic_line >>
		line_ending >>
		(
			line
		)
	)
);

named_attr!(#[doc=" Parse any token"],
	pub parse_token<CompleteStr<'_>, BasicToken>, alt!(
		parse_rem |
		parse_simple_instruction |
		parse_prefixed_instruction |
		parse_basic_value |
		parse_char
	)
);

named_attr!(#[doc=" Parse a comment"],
	pub parse_rem<CompleteStr<'_>, BasicToken>, do_parse!(
		sym: alt!(
			tag_no_case!("REM") => 
				{|_|{BasicTokenNoPrefix::Rem}} |
			char!('\'') => 
				{|_| {BasicTokenNoPrefix::SymbolQuote}}
		) >>
		list: take_till!(|ch|{ch==':' ||ch=='\n'}) >>
		(
			BasicToken::Comment(sym, list.as_bytes().to_vec())
		)
	)
);

named_attr!(#[doc=" Parse the instructions that do not need a prefix byte
	/// TODO Add all the other variants"],
	pub parse_simple_instruction<CompleteStr<'_>, BasicToken>, do_parse!(
		token: alt!(
			tag_no_case!("CALL") => {|_| BasicTokenNoPrefix::Call} |
			tag_no_case!("INPUT") => {|_| BasicTokenNoPrefix::Input} |
			tag_no_case!("PRINT") => {|_| BasicTokenNoPrefix::Print} 
		) >>
		(
			BasicToken::SimpleToken(token)
		)
	)
);

named_attr!(#[doc=" TODO add the missing chars"],
	pub parse_char<CompleteStr<'_>, BasicToken>, do_parse!(
		token: alt!(
			char!(':') => {|_| BasicTokenNoPrefix::StatementSeparator} |
			char!(' ') => {|_| BasicTokenNoPrefix::CharSpace} |

			char!('A') => {|_| BasicTokenNoPrefix::CharUpperA} |
			char!('B') => {|_| BasicTokenNoPrefix::CharUpperB} |
			char!('C') => {|_| BasicTokenNoPrefix::CharUpperC} |
			char!('D') => {|_| BasicTokenNoPrefix::CharUpperD} |
			char!('E') => {|_| BasicTokenNoPrefix::CharUpperE} |
			char!('F') => {|_| BasicTokenNoPrefix::CharUpperF} |
			char!('G') => {|_| BasicTokenNoPrefix::CharUpperG} |
			char!('H') => {|_| BasicTokenNoPrefix::CharUpperH} |
			char!('I') => {|_| BasicTokenNoPrefix::CharUpperI} |
			char!('J') => {|_| BasicTokenNoPrefix::CharUpperJ} |
			char!('K') => {|_| BasicTokenNoPrefix::CharUpperK} |
			char!('L') => {|_| BasicTokenNoPrefix::CharUpperL} |
			char!('M') => {|_| BasicTokenNoPrefix::CharUpperM} |
			char!('N') => {|_| BasicTokenNoPrefix::CharUpperN} |
			char!('O') => {|_| BasicTokenNoPrefix::CharUpperO} |
			char!('P') => {|_| BasicTokenNoPrefix::CharUpperP} |
			char!('Q') => {|_| BasicTokenNoPrefix::CharUpperQ} |
			char!('R') => {|_| BasicTokenNoPrefix::CharUpperR} |
			char!('S') => {|_| BasicTokenNoPrefix::CharUpperS} |
			char!('T') => {|_| BasicTokenNoPrefix::CharUpperT} |
			char!('U') => {|_| BasicTokenNoPrefix::CharUpperU} |
			char!('V') => {|_| BasicTokenNoPrefix::CharUpperV} |
			char!('W') => {|_| BasicTokenNoPrefix::CharUpperW} |
			char!('X') => {|_| BasicTokenNoPrefix::CharUpperX} |
			char!('Y') => {|_| BasicTokenNoPrefix::CharUpperY} |
			char!('Z') => {|_| BasicTokenNoPrefix::CharUpperZ} |

			char!('a') => {|_| BasicTokenNoPrefix::CharLowerA} |
			char!('b') => {|_| BasicTokenNoPrefix::CharLowerB} |
			char!('c') => {|_| BasicTokenNoPrefix::CharLowerC} |
			char!('d') => {|_| BasicTokenNoPrefix::CharLowerD} |
			char!('e') => {|_| BasicTokenNoPrefix::CharLowerE} |
			char!('f') => {|_| BasicTokenNoPrefix::CharLowerF} |
			char!('g') => {|_| BasicTokenNoPrefix::CharLowerG} |
			char!('h') => {|_| BasicTokenNoPrefix::CharLowerH} |
			char!('i') => {|_| BasicTokenNoPrefix::CharLowerI} |
			char!('j') => {|_| BasicTokenNoPrefix::CharLowerJ} |
			char!('k') => {|_| BasicTokenNoPrefix::CharLowerK} |
			char!('l') => {|_| BasicTokenNoPrefix::CharLowerL} |
			char!('m') => {|_| BasicTokenNoPrefix::CharLowerM} |
			char!('n') => {|_| BasicTokenNoPrefix::CharLowerN} |
			char!('o') => {|_| BasicTokenNoPrefix::CharLowerO} |
			char!('p') => {|_| BasicTokenNoPrefix::CharLowerP} |
			char!('q') => {|_| BasicTokenNoPrefix::CharLowerQ} |
			char!('r') => {|_| BasicTokenNoPrefix::CharLowerR} |
			char!('s') => {|_| BasicTokenNoPrefix::CharLowerS} |
			char!('t') => {|_| BasicTokenNoPrefix::CharLowerT} |
			char!('u') => {|_| BasicTokenNoPrefix::CharLowerU} |
			char!('v') => {|_| BasicTokenNoPrefix::CharLowerV} |
			char!('w') => {|_| BasicTokenNoPrefix::CharLowerW} |
			char!('x') => {|_| BasicTokenNoPrefix::CharLowerX} |
			char!('y') => {|_| BasicTokenNoPrefix::CharLowerY} |
			char!('z') => {|_| BasicTokenNoPrefix::CharLowerZ} 
		) >>
		(
			BasicToken::SimpleToken(token)
		)
	)
);

named_attr!(#[doc=" Parse the instructions that do not need a prefix byte
/// TODO Add all the other instructions"],
	pub parse_prefixed_instruction<CompleteStr<'_>, BasicToken>, do_parse!(
		token: alt!(
			tag_no_case!("ABS") => {|_| BasicTokenPrefixed::Abs}
		) >>
		(
			BasicToken::PrefixedToken(token)
		)
	)
);

named_attr!(#[doc=" Parse a basic value"],
	pub parse_basic_value<CompleteStr<'_>, BasicToken>, alt!(
		parse_hexadecimal_value_16bits |
		parse_decimal_value_16bits
	)
);

named_attr!(#[doc=" Parse an hexadecimal value"],
    pub parse_hexadecimal_value_16bits<CompleteStr<'_>, BasicToken>, do_parse!(
        tag_no_case!( "&") >>
        val: hex_u16_inner >>
        (
			BasicToken::Constant(
				BasicTokenNoPrefix::ValueIntegerHexadecimal16bits,
				BasicValue::new_integer(val)
			)
		)
        )
    );

named_attr!(#[doc=" TODO"],
    pub parse_decimal_value_16bits<CompleteStr<'_>, BasicToken>, do_parse!(
        val: dec_u16_inner >>
        (
			BasicToken::Constant(
				BasicTokenNoPrefix::ValueIntegerDecimal16bits,
				BasicValue::new_integer(val)
			)
		)
        )
    );

/// XXX stolen to the asm parser
#[inline]
pub fn hex_u16_inner(input: CompleteStr<'_>) -> IResult<CompleteStr<'_>, u16> {
    match is_a!(input, &b"0123456789abcdefABCDEF"[..]) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than  characters for a u16
            if parsed.input_len() > 4 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(16).unwrap_or(0);
                    res = value + (res * 16);
                }
                if res > u16::max_value() as u32 {
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
pub fn dec_u16_inner(input: CompleteStr<'_>) -> IResult<CompleteStr<'_>, u16> {
    match is_a!(input, &b"0123456789"[..]) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than 5 characters for a u16
            if parsed.input_len() > 5 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0u32;
                for e in parsed.iter_elements() {
                    let digit = e;
                    let value = digit.to_digit(10).unwrap_or(0);
                    res = value + (res * 10);
                }
                if res > u16::max_value() as u32 {
                    Err(::nom::Err::Error(error_position!(
                        input,
                        ErrorKind::Custom(0)
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
        assert!(dec_u16_inner(CompleteStr("10")).is_ok());

        match hex_u16_inner("1234".into()) {
            Ok((res, value)) => {
                println!("{:?}", &res);
                println!("{:x}", &value);
                assert_eq!(0x1234, value);
            }
            Err(e) => {
                panic!("{}", e);
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
                panic!("{}", e);
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
                panic!("{}", e);
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
                panic!("{}", e);
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
