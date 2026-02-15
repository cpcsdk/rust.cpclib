use cpclib_common::itertools::Itertools;
use cpclib_common::winnow;
use cpclib_common::winnow::combinator::{eof, terminated};
use cpclib_common::winnow::error::StrContext;
use cpclib_common::winnow::stream::{Offset, Stream};
use cpclib_common::winnow::{ModalResult, Parser};
use winnow::binary::{le_u16, u8};
use winnow::combinator::cut_err;

use crate::binary_parser::winnow::error::ContextError;
use crate::binary_parser::winnow::token::take;
use crate::tokens::{BasicTokenPrefixed, *};
use crate::{BasicLine, BasicProgram};

pub fn program(bytes: &mut &[u8]) -> ModalResult<BasicProgram, ContextError<StrContext>> {
    let initial_len = bytes.len();
    eprintln!("Starting parse with {} bytes", initial_len);

    let location = bytes.checkpoint();

    let mut lines = Vec::new();
    
    while bytes.offset_from(&location) < initial_len {
        let line = dbg!(line_or_end.parse_next(bytes))?;
        lines.push(line);
    }

    lines.pop(); // Remove the final None that indicates the end of the program
    let lines = lines.into_iter().flatten().collect_vec();


    eprintln!("Parsed {} lines, {} bytes remaining", lines.len(), bytes.len());
    
    if !bytes.is_empty() {
        eprintln!("WARNING: {} unconsumed bytes remaining after parsing!", bytes.len());
        eprintln!("First 20 bytes: {:?}", &bytes[0..bytes.len().min(20)]);
    }

    Ok(BasicProgram { lines })
}

// https://www.cpcwiki.eu/index.php?title=Technical_information_about_Locomotive_BASIC&mobileaction=toggle_view_desktop#Structure_of_a_BASIC_program
// Some(BasicLine) for a Line
// None for End
pub fn line_or_end(
    bytes: &mut &[u8]
) -> ModalResult<Option<BasicLine>, ContextError<StrContext>> {
    let length = cut_err(le_u16.context(StrContext::Label("Expecting a line length"))).parse_next(bytes)?;

    // leave if it is the end of the program
    if length == 0 {
        return Ok(None);
    }

    let line_number = cut_err(le_u16.context(StrContext::Label("Expecting a line number"))).parse_next(bytes)?;

    eprintln!("Parsing line {}, declared length: {}, remaining bytes: {}", line_number, length, bytes.len() + 4);
    
    let mut buffer = cut_err(take(length - 4).context(StrContext::Label("Wrong number of bytes")))
        .verify(|buffer: &[u8]| buffer[buffer.len() - 1] == 0)
        .context(StrContext::Label("Last byte should be 0"))
        .parse_next(bytes)?;

    eprintln!("  Buffer for line {} has {} bytes", line_number, buffer.len());
    
    let tokens = terminated(parse_tokens, eof).parse_next(&mut buffer)?;
    
    eprintln!("  Successfully parsed line {} with {} tokens", line_number, tokens.len());

    let line = BasicLine {
        line_number,
        forced_length: None,
        tokens
    };

    Ok(Some(line))
}

pub fn parse_tokens(bytes: &mut &[u8]) -> ModalResult<Vec<BasicToken>, ContextError<StrContext>> {
    let mut tokens = Vec::with_capacity(bytes.len());
    while !bytes.is_empty() {
        let code = BasicTokenNoPrefix::try_from(u8.parse_next(bytes)?).unwrap();

        match code {
            // Constant numbers 0-10 (no additional bytes needed)
            BasicTokenNoPrefix::ConstantNumber0 => {
                let value = BasicValue::new_integer_by_bytes(0, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber1 => {
                let value = BasicValue::new_integer_by_bytes(1, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber2 => {
                let value = BasicValue::new_integer_by_bytes(2, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber3 => {
                let value = BasicValue::new_integer_by_bytes(3, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber4 => {
                let value = BasicValue::new_integer_by_bytes(4, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber5 => {
                let value = BasicValue::new_integer_by_bytes(5, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber6 => {
                let value = BasicValue::new_integer_by_bytes(6, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber7 => {
                let value = BasicValue::new_integer_by_bytes(7, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber8 => {
                let value = BasicValue::new_integer_by_bytes(8, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber9 => {
                let value = BasicValue::new_integer_by_bytes(9, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            BasicTokenNoPrefix::ConstantNumber10 => {
                let value = BasicValue::new_integer_by_bytes(10, 0);
                let token = BasicToken::Constant(code, value);
                tokens.push(token);
            },
            
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

            // Variable definitions: token + 2-byte offset + variable name (last char has bit 7 set)
            BasicTokenNoPrefix::IntegerVariableDefinition | 
            BasicTokenNoPrefix::StringVariableDefinition | 
            BasicTokenNoPrefix::FloatingPointVariableDefinition |
            BasicTokenNoPrefix::VarUnknown1 |
            BasicTokenNoPrefix::VarUnknown2 |
            BasicTokenNoPrefix::VarUnknown3 |
            BasicTokenNoPrefix::CharTab |  // Documented as "var?" token
            BasicTokenNoPrefix::VarUnknown5 |
            BasicTokenNoPrefix::VariableDefinition1 | 
            BasicTokenNoPrefix::VariableDefinition2 | 
            BasicTokenNoPrefix::VariableDefinition3 => {
                // Read 2-byte offset (little-endian)
                let low = u8.parse_next(bytes)?;
                let high = u8.parse_next(bytes)?;
                let offset = BasicValue::new_integer_by_bytes(low, high);
                
                // Read variable name until we find a byte with bit 7 set to 1
                let mut name_bytes = Vec::new();
                loop {
                    let byte = u8.parse_next(bytes)?;
                    name_bytes.push(byte & 0x7F); // Clear bit 7 to get the actual character
                    if byte & 0x80 != 0 {
                        // Last character found
                        break;
                    }
                }
                
                let name = String::from_utf8_lossy(&name_bytes).to_string();
                let token = BasicToken::Variable(name, offset);
                tokens.push(token);
            },

            // Quoted string: token 0x22 + string content until closing quote or end of bytes
            BasicTokenNoPrefix::ValueQuotedString => {
                let mut string_bytes = Vec::new();
                let mut found_closing_quote = false;
                
                while !bytes.is_empty() {
                    // Peek at next byte to see if it's closing quote or end marker
                    if bytes[0] == 0x22 || bytes[0] == 0x00 {
                        if bytes[0] == 0x22 {
                            // Consume the closing quote
                            u8.parse_next(bytes)?;
                            found_closing_quote = true;
                        }
                        // If 0x00, leave it for EndOfTokenisedLine
                        break;
                    }
                    let byte = u8.parse_next(bytes)?;
                    string_bytes.push(byte);
                }
                
                // Track whether string had closing quote in binary format
                let token = BasicToken::CommentOrString(code, string_bytes, found_closing_quote);
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

