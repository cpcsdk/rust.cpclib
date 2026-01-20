use cpclib_common::itertools::Itertools;
use cpclib_common::winnow;
use cpclib_common::winnow::combinator::{eof, repeat, terminated};
use cpclib_common::winnow::{ModalResult, Parser};
use winnow::binary::{le_u16, u8};
use winnow::combinator::cut_err;

use crate::binary_parser::winnow::error::ContextError;
use crate::binary_parser::winnow::token::take;
use crate::tokens::{BasicTokenPrefixed, *};
use crate::{BasicLine, BasicProgram};

pub fn program(bytes: &mut &[u8]) -> ModalResult<BasicProgram, ContextError<&'static str>> {
    let lines: Vec<BasicLine> = repeat(0.., line_or_end.context("Error when parsing a basic line"))
        .verify(|lines: &Vec<Option<BasicLine>>| lines.last().map(|l| l.is_none()).unwrap_or(true))
        .map(|mut lines: Vec<Option<BasicLine>>| {
            lines.pop();
            lines.into_iter().map(|l| l.unwrap()).collect_vec()
        })
        .parse_next(bytes)?;

    Ok(BasicProgram { lines })
}

// https://www.cpcwiki.eu/index.php?title=Technical_information_about_Locomotive_BASIC&mobileaction=toggle_view_desktop#Structure_of_a_BASIC_program
// Some(BasicLine) for a Line
// None for End
pub fn line_or_end(
    bytes: &mut &[u8]
) -> ModalResult<Option<BasicLine>, ContextError<&'static str>> {
    let length = cut_err(le_u16.context("Expecting a line length")).parse_next(bytes)?;

    // leave if it is the end of the program
    if length == 0 {
        return Ok(None);
    }

    let line_number = cut_err(le_u16.context("Expecting a line number")).parse_next(bytes)?;

    dbg!("Tentative for line", line_number);

    let mut buffer = cut_err(take(length - 4).context("Wrong number of bytes"))
        .verify(|buffer: &[u8]| buffer[buffer.len() - 1] == 0)
        .context("Last byte should be 0")
        .parse_next(bytes)?;

    let tokens = terminated(parse_tokens, eof).parse_next(&mut buffer)?;

    let line = BasicLine {
        line_number,
        forced_length: None,
        tokens
    };

    dbg!(&line);
    dbg!(line.to_string());

    Ok(Some(line))
}

pub fn parse_tokens(bytes: &mut &[u8]) -> ModalResult<Vec<BasicToken>, ContextError<&'static str>> {
    let mut tokens = Vec::with_capacity(bytes.len());
    while !bytes.is_empty() {
        let code = BasicTokenNoPrefix::try_from(u8.parse_next(bytes)?).unwrap();

        match code {
            BasicTokenNoPrefix::ValueIntegerDecimal8bits => {
                let val = u8.parse_next(bytes)?;
                let value = BasicValue::new_integer_by_bytes(val, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },

            BasicTokenNoPrefix::ValueIntegerDecimal16bits |
            BasicTokenNoPrefix::ValueIntegerBinary16bits |
            BasicTokenNoPrefix::ValueIntegerHexadecimal16bits | 
            BasicTokenNoPrefix::LineNumber | 
            BasicTokenNoPrefix::LineMemoryAddressPointer => {
                let low = u8.parse_next(bytes)?;
                let high = u8.parse_next(bytes)?;
                let value = BasicValue::new_integer_by_bytes(low, high);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },

            BasicTokenNoPrefix::ValueFloatingPoint => {
                let b0 = u8.parse_next(bytes)?;
                let b1 = u8.parse_next(bytes)?;
                let b2 = u8.parse_next(bytes)?;
                let b3 = u8.parse_next(bytes)?;
                let b4 = u8.parse_next(bytes)?;

                let value = BasicValue::Float(BasicFloat::from_bytes([b0, b1, b2, b3, b4]));
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },

            BasicTokenNoPrefix::AdditionalTokenMarker => {
                let code2 = BasicTokenPrefixed::try_from(u8.parse_next(bytes)?).unwrap();
                let token = BasicToken::PrefixedToken(code2);
                tokens.push(token);
            },

            _ => {
                tokens.push(BasicToken::SimpleToken(code));
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer_8bits_decimal() {
        let expected_val = 100;
        let data = [
            BasicTokenNoPrefix::ValueIntegerDecimal8bits as u8, 
            expected_val
        ];
        
        // Use a slice reference as expected by the parser
        let mut input: &[u8] = &data;
        let res = parse_tokens.parse_next(&mut input).expect("Parsing failed");
        
        assert_eq!(res.len(), 1);
        match &res[0] {
             BasicToken::Constant(BasicTokenNoPrefix::ValueIntegerDecimal8bits, BasicValue::Integer(low, high)) => {
                assert_eq!(*low, expected_val);
                assert_eq!(*high, 0);
            },
            _ => panic!("Expected ValueIntegerDecimal8bits, got {:?}", res[0])
        }
    }

    #[test]
    fn test_parse_integer_16bits_decimal() {
        let expected_val: u16 = 1000; // 0x03E8
        let low = (expected_val % 256) as u8;
        let high = (expected_val / 256) as u8;

        let data = [
            BasicTokenNoPrefix::ValueIntegerDecimal16bits as u8, 
            low,
            high
        ];

        let mut input: &[u8] = &data;
        let res = parse_tokens.parse_next(&mut input).expect("Parsing failed");
        
        assert_eq!(res.len(), 1);
        match &res[0] {
             BasicToken::Constant(BasicTokenNoPrefix::ValueIntegerDecimal16bits, BasicValue::Integer(l, h)) => {
                assert_eq!(*l, low);
                assert_eq!(*h, high);
            },
            _ => panic!("Expected ValueIntegerDecimal16bits, got {:?}", res[0])
        }
    }

    #[test]
    fn test_parse_float() {
        // 0 value in Amstrad Basic Float: 0x00 0x00 0x00 0x00 0x00
        let data = [
            BasicTokenNoPrefix::ValueFloatingPoint as u8,
            0, 0, 0, 0, 0
        ];
        
        let mut input: &[u8] = &data;
        let res = parse_tokens.parse_next(&mut input).expect("Parsing failed");
        
        assert_eq!(res.len(), 1);
        match &res[0] {
             BasicToken::Constant(BasicTokenNoPrefix::ValueFloatingPoint, BasicValue::Float(f)) => {
                 // Check if it corresponds to 0.0
                 assert_eq!(f.to_f64(), 0.0);
            },
            _ => panic!("Expected ValueFloatingPoint, got {:?}", res[0])
        }    
    }

    #[test]
    fn test_parse_prefixed_token() {
         let data = [
             BasicTokenNoPrefix::AdditionalTokenMarker as u8,
             BasicTokenPrefixed::Abs as u8
         ];
         let mut input: &[u8] = &data;
         let res = parse_tokens.parse_next(&mut input).expect("Parsing failed");
         
         assert_eq!(res.len(), 1);
         assert_eq!(res[0], BasicToken::PrefixedToken(BasicTokenPrefixed::Abs));
    }

    #[test]
    fn test_sequence() {
        let prefix = BasicTokenNoPrefix::AdditionalTokenMarker as u8;
        let abs = BasicTokenPrefixed::Abs as u8;
        let space = BasicTokenNoPrefix::CharSpace as u8;
        
        let data = [
            space,
            prefix, abs
        ];
        
        let mut input: &[u8] = &data;
        let res = parse_tokens.parse_next(&mut input).expect("Parsing failed");
        
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], BasicToken::SimpleToken(BasicTokenNoPrefix::CharSpace));
        assert_eq!(res[1], BasicToken::PrefixedToken(BasicTokenPrefixed::Abs));
    }
}
