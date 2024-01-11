use cpclib_common::winnow::{PResult, Parser, combinator::terminated, stream::Stream};
use cpclib_common::winnow::combinator::{opt, cut_err};

use crate::preamble::{parse_expr, located_expr, my_space0, my_space1, parse_single_token, LocatedListing, one_instruction_inner_code};

use super::{InnerZ80Span, Z80ParserError, LocatedTokenInner, LocatedToken, inner_code};


pub static STAND_ALONE_DIRECTIVE_ORGAMS: &[&[u8]] = &[
    b"BANK",
    b"BRK",
    b"BYTE",
    b"DEFS",
    b"ELSE",
  //  b"END",
    b"ENT",
    b"IMPORT",
    b"ORG",
    b"SKIP",
    b"WORD",
];

pub static START_DIRECTIVE_ORGAMS: &[&[u8]] = &[
    b"IF",
    b"MACRO",
];

pub static END_DIRECTIVE_ORGAMS: &[&[u8]] = &[
    b"END", 
    b"ENDM",
    b"]"
];


pub fn parse_orgams_repeat( input: &mut InnerZ80Span)  -> PResult<LocatedToken, Z80ParserError> {
    let input_start = input.checkpoint();

	let amount = terminated(
		located_expr,
		(my_space0, "**", my_space0)
	).parse_next(input)?;


    let bracket = opt('[').parse_next(input)?;
    let listing = if bracket.is_some() {
        my_space0.parse_next(input)?;
        let listing = cut_err(inner_code.context("ORGAMS REPEAT: unable to parse inner code")).parse_next(input)?;
        ']'.parse_next(input)?;
        listing
    } else {
        one_instruction_inner_code.parse_next(input)?
    };


	let token = LocatedTokenInner::Repeat(amount, listing, None, None, None);
    let token = token.into_located_token_between(input_start, input.clone());

	Ok(token)
}