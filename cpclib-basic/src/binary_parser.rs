use cpclib_common::itertools::Itertools;
use cpclib_common::winnow;
use cpclib_common::winnow::combinator::{eof, repeat, terminated};
use cpclib_common::winnow::{PResult, Parser};
use winnow::binary::{le_u16, u8};
use winnow::combinator::cut_err;

use crate::binary_parser::winnow::error::ContextError;
use crate::binary_parser::winnow::token::take;
use crate::tokens::{BasicTokenPrefixed, *};
use crate::{BasicLine, BasicProgram};

pub fn program(bytes: &mut &[u8]) -> PResult<BasicProgram, ContextError<&'static str>> {
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
pub fn line_or_end(bytes: &mut &[u8]) -> PResult<Option<BasicLine>, ContextError<&'static str>> {
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

pub fn parse_tokens(bytes: &mut &[u8]) -> PResult<Vec<BasicToken>, ContextError<&'static str>> {
    let mut tokens = Vec::with_capacity(bytes.len());
    while !bytes.is_empty() {
        let code = BasicTokenNoPrefix::try_from(u8.parse_next(bytes)?).unwrap();

        match code {
            BasicTokenNoPrefix::ValueIntegerDecimal8bits => {
                todo!()
            },

            BasicTokenNoPrefix::ValueIntegerDecimal16bits => {
                todo!()
            },

            BasicTokenNoPrefix::ValueIntegerBinary16bits => {
                todo!()
            },

            BasicTokenNoPrefix::ValueIntegerHexadecimal16bits => {
                let low = u8.parse_next(bytes)?;
                let high = u8.parse_next(bytes)?;
                let value = BasicValue::new_integer_by_bytes(low, high);
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
