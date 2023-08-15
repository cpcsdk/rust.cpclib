#![allow(clippy::cast_lossless)]

use std::ops::Deref;
use std::sync::Arc;

use cpclib_common::itertools::Itertools;
use cpclib_common::nom::branch::*;
use cpclib_common::nom::bytes::complete::{tag, tag_no_case, *};
use cpclib_common::nom::character::complete::*;
use cpclib_common::nom::combinator::*;
use cpclib_common::nom::error::*;
use cpclib_common::nom::lib::std::convert::Into;
use cpclib_common::nom::multi::{separated_list1, *};
use cpclib_common::nom::sequence::*;
#[allow(missing_docs)]
use cpclib_common::nom::*;
use cpclib_common::nom_locate::LocatedSpan;
#[cfg(not(target_arch = "wasm32"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use cpclib_common::smol_str::SmolStr;
use cpclib_common::{bin_number, dec_number, hex_number, lazy_static};
use cpclib_sna::parse::{parse_flag, parse_flag_value};
use cpclib_sna::{FlagValue, SnapshotVersion};
use cpclib_tokens::ListingElement;
use crc::*;
use either::Either;

use super::context::*;
use super::obtained::*;
use super::*;
use crate::preamble::*;

const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Z80ParserErrorKind {
    /// Static string added by the `context` function
    Context(&'static str),
    /// Indicates which character was expected by the `char` function
    Char(char),
    /// Error kind given by various nom parsers
    Nom(ErrorKind),
    /// Chain of errors provided by an inner listing
    Inner {
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Z80ParserError(Vec<(Z80Span, Z80ParserErrorKind)>);

impl Z80ParserError {
    pub fn errors(&self) -> Vec<(&Z80Span, &Z80ParserErrorKind)> {
        let mut res = Vec::new();

        for e in self.0.iter() {
            if let Z80ParserErrorKind::Inner { listing: _, error } = &e.1 {
                res.extend(error.errors())
            }
            else {
                res.push((&e.0, &e.1));
            }
        }
        res
    }
}

impl From<ErrorKind> for Z80ParserErrorKind {
    fn from(other: ErrorKind) -> Self {
        Self::Nom(other)
    }
}

impl From<char> for Z80ParserErrorKind {
    fn from(other: char) -> Self {
        Self::Char(other)
    }
}

impl From<VerboseErrorKind> for Z80ParserErrorKind {
    fn from(other: VerboseErrorKind) -> Self {
        match other {
            VerboseErrorKind::Context(ctx) => Self::Context(ctx),
            VerboseErrorKind::Char(c) => Self::Char(c),
            VerboseErrorKind::Nom(n) => n.into()
        }
    }
}
impl From<VerboseError<Z80Span>> for Z80ParserError {
    fn from(other: VerboseError<Z80Span>) -> Self {
        Self(
            other
                .errors
                .into_iter()
                .map(|(i, k)| (i, k.into()))
                .collect_vec()
        )
    }
}

impl Z80ParserError {
    pub fn from_inner_error(
        input: Z80Span,
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    ) -> Self {
        Self(vec![(input, Z80ParserErrorKind::Inner { listing, error })])
    }
}

impl ParseError<Z80Span> for Z80ParserError {
    fn from_error_kind(input: Z80Span, kind: ErrorKind) -> Self {
        Self(vec![(input, kind.into())])
    }

    fn from_char(input: Z80Span, c: char) -> Self {
        Self(vec![(input, c.into())])
    }

    fn append(input: Z80Span, kind: ErrorKind, mut other: Self) -> Self {
        other.0.push((input, kind.into()));
        other
    }
}

impl ContextError<Z80Span> for Z80ParserError {
    fn add_context(input: Z80Span, ctx: &'static str, mut other: Self) -> Self {
        other.0.push((input, Z80ParserErrorKind::Context(ctx)));
        other
    }
}

/// ...
pub mod error_code {
    /// ...
    pub const ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION: u32 = 128;
    /// ...
    pub const INVALID_ARGUMENT: u32 = 129;
    /// ...
    pub const UNABLE_TO_PARSE_INNER_CONTENT: u32 = 130;
}

// TODO search why they are listed to forbid label naming. Delete it if unneeded
const REGISTERS: &[&str] = &["AF", "HL", "DE", "BC", "IX", "IY", "IXL", "IXH"];

const INSTRUCTIONS: &[&str] = &[
    "ADC", "ADD", "AND", "BIT", "CALL", "CCF", "CP", "CPD", "CPDR", "CPI", "CPIR", "CPL", "DAA",
    "DEC", "DI", "DJNZ", "EI", "EX", "EXX", "HALT", "IM", "IN", "INC", "IND", "INDR", "INI",
    "INIR", "JP", "JR", "LD", "LDD", "LDDR", "LDI", "LDIR", "NEG", "NOP", "OR", "OTDR", "OTIR",
    "OUT", "OUTD", "OUTI", "POP", "PUSH", "RES", "RET", "RETI", "RETN", "RL", "RLA", "RLC", "RLCA",
    "RLD", "RR", "RRA", "RRC", "RRCA", "RRD", "RST", "SBC", "SCF", "SET", "SLA", "SRA", "SRL",
    "SUB", "XOR", "SL1", "SLL", "EXA", "EXD"
];

const STAND_ALONE_DIRECTIVE: &[&str] = &[
    "ALIGN",
    "ASSERT",
    "BANK",
    "BANKSET",
    "BINCLUDE",
    "BREAK",
    "BREAKPOINT",
    "BUILDSNA",
    "BYTE",
    "CASE",
    "CHARSET",
    "DB",
    "DEFAULT",
    "DEFB",
    "DEFM",
    "DEFS",
    "DEFW",
    "DEFSECTION",
    "DM",
    "DS",
    "DW",
    "ELSE",
    "END",
    "EQU",
    "EXPORT",
    "FAIL",
    "INCBIN",
    "INCLUDE",
    "INCLZ4",
    "INCEXO",
    "INCL48",
    "INCL49",
    "INCAPU",
    "INCZX0",
    "LET",
    "LIMIT",
    "LIST",
    "LZEXO",
    "MODULE",
    "NOEXPORT",
    "NOLIST",
    "NOP",
    "ORG",
    "PAUSE",
    "PRINT",
    "PROTECT",
    "RANGE",
    "READ",
    "REND",
    "REPEAT",
    "RORG",
    "RETURN",
    "RUN",
    "SAVE",
    "SECTION",
    "SNAINIT",
    "SNAPINIT",
    "SNASET",
    "STR",
    "TEXT",
    "TICKER",
    "UNDEF",
    "UNTIL",
    "WAITNOPS",
    "WORD",
    "WRITE DIRECT",
    "WRITE"
];

const START_DIRECTIVE: &[&str] = &[
    "CONFINED", "FOR", "IF", "IFDEF", "IFNDEF", "IFUSED", "ITER", "ITERATE", "LZ4", "LZ48", "LZ49",
    "LZ48", "LZAPU", "LZX0", "LZEXO", "LZ4", "LZX7", "MACRO", "MODULE", "PHASE", "REPEAT", "REPT",
    "STRUCT", "SWITCH", "WHILE"
];

// This table is supposed to contain the keywords that finish a section
const END_DIRECTIVE: &[&str] = &[
    "BREAK",
    "CASE",
    "CEND",
    "DEFAULT",
    "DEPHASE",
    "ELSE",
    "ENDC",
    "ENDCONFINED",
    "ENDF",
    "ENDFOR",
    "ENDFUNCTION",
    "ENDI",
    "ENDIF",
    "ENDIF", // if directive
    "ENDITER",
    "ENDITERATE",
    "ENDM",
    "ENDMACRO",
    "ENDMODULE",
    "ENDR",
    "ENDREP", // repeat directive
    "ENDREPEAT",
    "ENDS",
    "ENDSWITCH",
    "ENDW",
    "FEND",
    "IEND",
    "LZCLOSE",
    "REND", // rorg directive
    "UNTIL",
    "WEND"
];

// tODO use hash-based structures
lazy_static::lazy_static! {
    static ref _DOTTED_STAND_ALONE_DIRECTIVE: Vec<String> = STAND_ALONE_DIRECTIVE
                                                .iter()
                                                .map(|d| format!(".{}", d))
                                                .collect_vec();
    static ref _DOTTED_START_DIRECTIVE: Vec<String> = START_DIRECTIVE
                                                .iter()
                                                .map(|d| format!(".{}", d))
                                                .collect_vec();
    static ref _DOTTED_END_DIRECTIVE: Vec<String> = END_DIRECTIVE
                                                .iter()
                                                .map(|d| format!(".{}", d))
                                                .collect_vec();
    static ref DOTTED_STAND_ALONE_DIRECTIVE: Vec<&'static str> = _DOTTED_STAND_ALONE_DIRECTIVE.iter().map(String::as_str).collect_vec();
    static ref DOTTED_START_DIRECTIVE: Vec<&'static str> = _DOTTED_START_DIRECTIVE.iter().map(String::as_str).collect_vec();
    static ref DOTTED_END_DIRECTIVE: Vec<&'static str> = _DOTTED_END_DIRECTIVE.iter().map(String::as_str).collect_vec();


    static ref DOTTED_IMPOSSIBLE_NAMES: Vec<&'static str> = REGISTERS
        .into_iter()
        .chain(INSTRUCTIONS.into_iter())
        .chain(DOTTED_STAND_ALONE_DIRECTIVE.iter())
        .chain(DOTTED_START_DIRECTIVE.iter())
        .chain(DOTTED_END_DIRECTIVE.iter())
        .cloned()
        .collect();

    static ref IMPOSSIBLE_NAMES: Vec<&'static str> = REGISTERS
        .into_iter()
        .chain(INSTRUCTIONS.into_iter())
        .chain(STAND_ALONE_DIRECTIVE.into_iter())
        .chain(START_DIRECTIVE.into_iter())
        .chain(END_DIRECTIVE.into_iter())
        .cloned()
        .collect();

    static ref MIN_MAX_LABEL_SIZE: (usize, usize) = DOTTED_IMPOSSIBLE_NAMES.iter().map(|l| l.len()).minmax().into_option().unwrap();
    static ref DOTTED_MIN_MAX_LABEL_SIZE:  (usize, usize) = DOTTED_IMPOSSIBLE_NAMES.iter().map(|l| l.len()).minmax().into_option().unwrap();

}

/// Produce the stream of tokens. In case of error, return an explanatory string.
/// In case of success loop over all the tokens in order to expand those that read files
pub fn parse_z80_with_context_builder<S: Into<String>>(
    str: S,
    builder: ParserContextBuilder
) -> Result<LocatedListing, AssemblerError> {
    let res = LocatedListing::new_complete_source(str.into(), builder)
        .map_err(|l| AssemblerError::LocatedListingError(std::sync::Arc::new(l)));

    res
}

/// TODO better to build parse_z80_with_options from parse_z80_span than the opposite
// pub fn parse_z80_span(span: Z80Span) -> Result<LocatedListing, AssemblerError> {
//    let ctx = span.extra.clone();
//    parse_z80_with_options(span.as_str(), ctx)
//}

#[inline]
pub fn parse_z80<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_str(code)
}

/// Parse a string and return the corresponding listing
pub fn parse_z80_str<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_with_context_builder(code, ParserContextBuilder::default())
}

#[inline]
/// nom many0 does not seem to fit our parser requirements
pub fn my_many0<O, E, F>(mut f: F) -> impl FnMut(Z80Span) -> IResult<Z80Span, Vec<O>, E>
where
    F: Parser<Z80Span, O, E>,
    E: ParseError<Z80Span>
{
    #[inline]
    move |mut i: Z80Span| {
        let mut acc;

        match f.parse(i.clone()) {
            Err(Err::Error(_)) => return Ok((i, Vec::with_capacity(0))),
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                if i1 == i {
                    return Ok((i, Vec::with_capacity(0))); // diff is here
                }
                acc = Some(Vec::with_capacity(2)); // allocated only if used
                i = i1;
                acc.as_mut().map(|acc| acc.push(o));
            }
        }

        let mut acc = acc.unwrap();
        loop {
            match f.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, acc)),
                Err(e) => return Err(e),
                Ok((i1, o)) => {
                    if i1 == i {
                        return Ok((i, acc)); // diff is here
                    }

                    i = i1;
                    acc.push(o);
                }
            }
        }
    }
}

#[inline]
pub fn my_many0_in<'vec, O, E, F>(
    mut f: F,
    r#in: &'vec mut Vec<O>
) -> impl FnMut(Z80Span) -> IResult<Z80Span, (), E> + 'vec
where
    F: Parser<Z80Span, O, E> + 'vec,
    E: ParseError<Z80Span>
{
    #[inline]
    move |mut i: Z80Span| {
        loop {
            match f.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, ())),
                Err(e) => return Err(e),
                Ok((i1, o)) => {
                    if i1 == i {
                        return Ok((i, ())); // diff is here
                    }

                    i = i1;
                    r#in.push(o);
                }
            }
        }
    }
}

#[inline]
fn my_separated_list0_in<'vec, I, O, O2, E, F, G>(
    mut sep: G,
    mut f: F,
    r#in: &'vec mut Vec<O>
) -> impl FnMut(I) -> IResult<I, (), E> + 'vec
where
    I: Clone + InputLength,
    F: Parser<I, Either<O, Vec<O>>, E> + 'vec,
    G: Parser<I, O2, E> + 'vec,
    E: ParseError<I>
{
    #[inline]
    move |mut i: I| {
        match f.parse(i.clone()) {
            Err(Err::Error(_)) => return Ok((i, ())),
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                match o {
                    Either::Left(o) => {
                        r#in.push(o);
                    }
                    Either::Right(mut os) => {
                        r#in.append(&mut os);
                    }
                }
                i = i1;
            }
        }

        loop {
            let len = i.input_len();
            match sep.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, ())),
                Err(e) => return Err(e),
                Ok((i1, _)) => {
                    // infinite loop check: the parser must always consume
                    if i1.input_len() == len {
                        return Err(Err::Error(E::from_error_kind(i1, ErrorKind::SeparatedList)));
                    }

                    match f.parse(i1.clone()) {
                        Err(Err::Error(_)) => return Ok((i, ())),
                        Err(e) => return Err(e),
                        Ok((i2, o)) => {
                            match o {
                                Either::Left(o) => {
                                    r#in.push(o);
                                }
                                Either::Right(mut os) => {
                                    r#in.append(&mut os);
                                }
                            }

                            i = i2;
                        }
                    }
                }
            }
        }
    }
}

#[inline]
pub fn my_many0_nocollect<O, E, F>(mut f: F) -> impl FnMut(Z80Span) -> IResult<Z80Span, (), E>
where
    F: Parser<Z80Span, O, E>,
    E: ParseError<Z80Span>
{
    #[inline]
    move |mut i: Z80Span| {
        loop {
            match f.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, ())),
                Err(e) => return Err(e),
                Ok((i1, _)) => {
                    if i1 == i {
                        return Ok((i, ())); // diff is here
                    }

                    i = i1;
                }
            }
        }
    }
}

#[inline]
pub fn my_many_till_nocollect<I, O, P, E, F, G>(
    mut f: F,
    mut g: G
) -> impl FnMut(I) -> IResult<I, ((), P), E>
where
    I: Clone + InputLength,
    F: Parser<I, O, E>,
    G: Parser<I, P, E>,
    E: ParseError<I>
{
    #[inline]
    move |mut i: I| {
        loop {
            let len = i.input_len();
            match g.parse(i.clone()) {
                Ok((i1, o)) => return Ok((i1, ((), o))),
                Err(Err::Error(_)) => {
                    match f.parse(i.clone()) {
                        Err(Err::Error(err)) => {
                            return Err(Err::Error(E::append(i, ErrorKind::ManyTill, err)))
                        }
                        Err(e) => return Err(e),
                        Ok((i1, _o)) => {
                            // infinite loop check: the parser must always consume
                            if i1.input_len() == len {
                                return Err(Err::Error(E::from_error_kind(
                                    i1,
                                    ErrorKind::ManyTill
                                )));
                            }

                            i = i1;
                        }
                    }
                }
                Err(e) => return Err(e)
            }
        }
    }
}

#[inline]
pub fn my_many1_nocollect<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
where
    I: Clone + InputLength,
    F: Parser<I, O, E>,
    E: ParseError<I>
{
    #[inline]
    move |mut i: I| {
        match f.parse(i.clone()) {
            Err(Err::Error(err)) => Err(Err::Error(E::append(i, ErrorKind::Many1, err))),
            Err(e) => Err(e),
            Ok((i1, _o)) => {
                i = i1;

                loop {
                    let len = i.input_len();
                    match f.parse(i.clone()) {
                        Err(Err::Error(_)) => return Ok((i, ())),
                        Err(e) => return Err(e),
                        Ok((i1, _o)) => {
                            // infinite loop check: the parser must always consume
                            if i1.input_len() == len {
                                return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many1)));
                            }

                            i = i1;
                        }
                    }
                }
            }
        }
    }
}

#[inline]
fn inner_code(input: Z80Span) -> IResult<Z80Span, LocatedListing, Z80ParserError> {
    inner_code_with_state(input.extra.state.clone())(input)
}

/// Workaround because many0 is not used in the main root function
/// TODO add an argument to handle context change
#[inline]
pub fn inner_code_with_state(
    new_state: ParsingState
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedListing, Z80ParserError> {
    move |input: Z80Span| {
        LocatedListing::parse_inner(input, new_state)
            .map(|(i, l)| (i, Arc::<LocatedListing>::try_unwrap(l).unwrap()))
    }
}

/// TODO
pub fn parse_rorg(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let (input, _) = space0(input)?;
    let rorg_start = input.clone();
    let (input, _) = alt((tag_no_case("PHASE"), tag_no_case("RORG")))(input)?;

    let (input, exp) = delimited(space1, located_expr, space0)(input)?;

    let (input, _) = my_line_ending(input)?;

    let (input, inner) = inner_code(input)?;

    let (input, _) = preceded(space0, alt((tag_no_case("DEPHASE"), tag_no_case("REND"))))(input)?;

    Ok((input.clone(), LocatedToken::Rorg(exp, inner, rorg_start)))
}

/// TODO - limit the listing possibilities
pub fn parse_function_listing(input: Z80Span) -> IResult<Z80Span, LocatedListing, Z80ParserError> {
    inner_code_with_state(ParsingState::FunctionLimited)(input)
}

pub fn parse_function(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let function_start = input.clone();
    let (input, _) = preceded(space0, parse_directive_word("FUNCTION"))(input)?;
    let (input, name) = cut(context("FUNCTION: wrong name", parse_label(false)))(input)?; // TODO use a specific function for that

    let (input, arguments) = cut(context(
        "FUNCTION: wrong parameters",
        preceded(
            opt(parse_comma), // comma after macro name is not mandatory
            separated_list0(
                parse_comma,
                // parse_label(false)
                delimited(
                    my_space0,
                    take_till1(|c| c == '\n' || c == '\r' || c == ':' || c == ',' || c == ' '),
                    my_space0
                )
            )
        )
    ))(input)?;

    let (input, _) = preceded(space0, my_line_ending)(input)?;
    let (before_expr, listing) =
        cut(context("FUNCTION: invalid content", parse_function_listing))(input)?;

    let (input, _) = my_many0_nocollect(my_line_ending)(before_expr)?;
    let (input, _) = alt((
        parse_directive_word("ENDF"),
        parse_directive_word("ENDFUNCTION")
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Function(
            name,
            arguments,
            listing,
            function_start.slice(..function_start.len() - input.len())
        )
    ))
}

/// TODO
pub fn parse_macro(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let dir_start = input.clone();
    let (input, _) = preceded(space0, parse_directive_word("MACRO"))(input)?;

    // macro name
    let (input, name) = cut(context("MACRO: wrong name", parse_label(false)))(input)?; // TODO use a specific function for that

    // macro arguments
    let (input, arguments) = preceded(
        opt(parse_comma), // comma after macro name is not mandatory
        separated_list0(
            parse_comma,
            // parse_label(false)
            delimited(
                space0,
                verify(
                    take_till(|c| c == '\n' || c == '\r' || c == ':' || c == ',' || c == ' '),
                    |s: &Z80Span| !s.is_empty()
                ),
                space0
            )
        )
    )(input)?;

    let (input, _) = alt((space0, my_line_ending))(input)?;
    let before_content = input.clone();
    let (input, content) = cut(context(
        "MACRO: issue in the content",
        many_till(
            take(1usize),
            alt((
                parse_directive_word("ENDM"),
                parse_directive_word("ENDMACRO"),
                parse_directive_word("MEND")
            ))
        )
    ))(input)?;

    let content = before_content.take(content.0.len());

    let span = dir_start.take(dir_start.input_len() - input.input_len());
    Ok((
        input.clone(),
        LocatedToken::Macro {
            name,
            params: arguments,
            content,
            span
        }
    ))
}

/// TODO
pub fn parse_while(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let (input, _) = space0(input)?;
    let while_start = input.clone();
    let (input, _) = parse_directive_word("WHILE")(input)?;

    let (input, cond) = cut(context("WHILE: error in condition", located_expr))(input)?;

    // we must have either a new line or :
    let (input, _) = alt((
        delimited(space0, tag(":"), space0),
        preceded(space0, line_ending)
    ))(input)?;

    let (input, inner) = cut(context("WHILE: issue in the content", inner_code))(input)?;
    let (input, _) = cut(context(
        "WHILE: not closed",
        preceded(
            space0,
            alt((parse_directive_word("ENDW"), parse_directive_word("WEND")))
        )
    ))(input)?;

    Ok((input.clone(), LocatedToken::While(cond, inner, while_start)))
}

pub fn parse_module(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let module_start = input.clone();
    let (input, _) = parse_directive_word("MODULE")(input)?;

    let (input, name) = cut(context("MODULE: error in naming", parse_label(false)))(input)?;

    let (input, inner) = cut(context("MODULE: issue in the content", inner_code))(input)?;
    let (input, _) = cut(context(
        "MODULE: not closed",
        preceded(space0, parse_directive_word("ENDMODULE"))
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Module(name, inner, module_start)
    ))
}

/// Parse a sub-listing part that aims at being crunched after being assembled at first pass
pub fn parse_crunched_section(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let crunched_start = input.clone();
    let (input, kind) = preceded(
        space0,
        alt((
            map(parse_directive_word("LZEXO"), |_| CrunchType::LZEXO),
            map(parse_directive_word("LZ4"), |_| CrunchType::LZ4),
            map(parse_directive_word("LZ48"), |_| CrunchType::LZ48),
            map(parse_directive_word("LZ49"), |_| CrunchType::LZ49),
            map(parse_directive_word("LZX7"), |_| CrunchType::LZX7),
            map(parse_directive_word("LZX0"), |_| CrunchType::LZX0),
            map(parse_directive_word("LZAPU"), |_| CrunchType::LZAPU)
        ))
    )(input)?;

    let (input, inner) = cut(context(
        "CRUNCHED SECTION: issue in the content",
        inner_code
    ))(input)?;

    let (input, _) = cut(context(
        "CRUNCHED SECTION section: not closed",
        tuple((space0, parse_directive_word("LZCLOSE"), space0))
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::CrunchedSection(kind, inner, crunched_start)
    ))
}

/// Parse the switch directive
pub fn parse_switch(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let (switch_start, _) = my_many0_nocollect(alt((space1, my_line_ending)))(input)?;
    let (input, _) = parse_directive_word("SWITCH")(switch_start.clone())?;

    let (input, value) = cut(context(
        "SWITCH: tested value",
        preceded(space0, located_expr)
    ))(input)?;

    let mut cases_listing = Vec::new();
    let mut default_listing = None;

    let mut loop_start = input;
    loop {
        let (input, _) = cut(context(
            "SWITCH: whitespace error",
            my_many0_nocollect(alt((
                space1,
                line_ending,
                tag(":"),
                recognize(parse_comment)
            )))
        ))(loop_start)?;

        // after default it is mandatory to end the block
        let (input, endswitch) = if default_listing.is_some() {
            cut(context(
                "SWITCH: endswitch not present after default listing.",
                preceded(
                    space0,
                    map(
                        alt((
                            parse_directive_word("ENDS"),
                            parse_directive_word("ENDSWITCH")
                        )),
                        |_| true
                    )
                )
            ))(input)?
        }
        else {
            preceded(
                space0,
                map(
                    opt(alt((
                        parse_directive_word("ENDS"),
                        parse_directive_word("ENDSWITCH")
                    ))),
                    |e| e.is_some()
                )
            )(input)?
        };
        if endswitch {
            return Ok((
                input,
                LocatedToken::Switch(value, cases_listing, default_listing, switch_start.clone())
            ));
        }

        let (input, value) = preceded(my_space0, opt(parse_directive_word("CASE")))(input)?;
        loop_start = if value.is_some() {
            let (input, value) = cut(context(
                "SWITCH: case value error.",
                delimited(space0, located_expr, opt(tag(":")))
            ))(input)?;

            let (input, inner) = cut(context("SWITCH: error in case code", inner_code))(input)?;

            let (input, do_break) = opt(preceded(space0, parse_directive_word("BREAK")))(input)?;

            cases_listing.push((value, inner, do_break.is_some()));
            input
        }
        else {
            let (input, _) = cut(context(
                "Only CASE, DEFAULT or ENDSWITCH are expected.",
                delimited(
                    space0,
                    parse_directive_word("DEFAULT"),
                    opt(pair(space0, tag(":")))
                )
            ))(input)?;
            let (input, default) =
                cut(context("SWITCH: error in default case", inner_code))(input)?;
            default_listing = Some(default);
            input
        }
    }
}

pub fn parse_for(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let for_start = input.clone();
    let (input, _) = preceded(space0, parse_directive_word("FOR"))(input)?;

    // Get parameters
    let (input, counter) = cut(parse_label(false))(input)?;
    let (input, start) = cut(preceded(parse_comma, located_expr))(input)?;
    let (input, stop) = cut(preceded(parse_comma, located_expr))(input)?;
    let (input, step) = opt(preceded(parse_comma, located_expr))(input)?;

    // Get loop content
    let (input, inner) = cut(context("FOR: issue in the content", inner_code))(input)?;

    // Collect end of loop
    let (input, _) = cut(context(
        "FOR: not closed",
        preceded(
            space0,
            alt((
                parse_directive_word("ENDFOR"),
                parse_directive_word("FEND"),
                parse_directive_word("ENDF")
            ))
        )
    ))(input)?;

    let for_span = for_start.take(for_start.input_len() - input.input_len());
    Ok((
        input.clone(),
        LocatedToken::For {
            label: counter,
            start,
            stop,
            step,
            listing: inner,
            span: for_span
        }
    ))
}

pub fn parse_confined(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let (input, _) = space0(input)?;
    let confined_start = input.clone();

    let (input, _) = parse_directive_word("CONFINED")(input)?;

    let (input, inner) = cut(context("CONFINED: issue in the content", inner_code))(input)?;

    let (input, _) = cut(context(
        "CONFINED: not closed",
        preceded(
            space0,
            alt((
                parse_directive_word("ENDCONFINED"),
                parse_directive_word("CEND"),
                parse_directive_word("ENDC")
            ))
        )
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Confined(
            inner,
            confined_start.take(confined_start.input_len() - input.input_len())
        )
    ))
}

pub fn parse_repeat(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let repeat_start = input.clone();
    let (input, _) = preceded(
        space0,
        alt((
            parse_directive_word("REP"),
            parse_directive_word("REPT"),
            parse_directive_word("REPEAT")
        ))
    )(input)?;

    let (input, count) = opt(located_expr)(input)?;
    match count {
        Some(count) => {
            let (input, counter) = cut(context(
                "REPEAT: issue in the counter",
                opt(preceded(parse_comma, parse_label(false)))
            ))(input)?;
            let (input, counter_start) = opt(preceded(parse_comma, located_expr))(input)?;
            let (input, inner) = cut(context("REPEAT: issue in the content", inner_code))(input)?;

            let (input, _) = cut(context(
                "REPEAT: not closed",
                preceded(
                    space0,
                    alt((
                        parse_directive_word("ENDREPEAT"),
                        parse_directive_word("ENDREPT"),
                        parse_directive_word("ENDREP"),
                        parse_directive_word("ENDR"),
                        parse_directive_word("REND")
                    ))
                )
            ))(input)?;

            Ok((
                input.clone(),
                LocatedToken::Repeat(
                    count,
                    inner,
                    counter,
                    counter_start,
                    repeat_start.take(repeat_start.input_len() - input.input_len())
                )
            ))
        }

        None => {
            let (input, inner) = cut(context("REPEAT: issue in the content", inner_code))(input)?;

            let (input, _) = cut(context(
                "REPEAT ... UNTIL: not closed",
                delimited(space0, parse_directive_word("UNTIL"), space0)
            ))(input)?;
            let (input, cond) = cut(context("REPEAT UNTIL: condition error", located_expr))(input)?;
            Ok((
                input.clone(),
                LocatedToken::RepeatUntil(cond, inner, repeat_start)
            ))
        }
    }
}

pub fn parse_iterate(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let iterate_start = input.clone();
    let (input, _) = preceded(
        space0,
        alt((
            parse_directive_word("ITERATE"),
            parse_directive_word("ITER")
        ))
    )(input)?;

    let (input, counter) = cut(context(
        "ITERATE: issue in the counter",
        preceded(space0, parse_label(false))
    ))(input)?;

    let (input, comma_or_in) = cut(context(
        "ITERATE: expected ',' or 'in'",
        preceded(my_space0, alt((parse_word("IN"), parse_comma)))
    ))(input)?;

    let (input, values) = if comma_or_in.contains(",") {
        let (input, values) = cut(context("ITERATE: values issue", expr_list))(input)?;
        (input, either::Either::Left(values))
    }
    else {
        let (input, values) = cut(context(
            "ITERATE: list issue",
            alt((
                parse_expr_bracketed_list,
                parse_unary_function_call,
                parse_binary_function_call,
                parse_any_function_call,
                parse_assemble,
                map(parse_label(false), |l| LocatedExpr::Label(l))
            ))
        ))(input)?;
        (input, either::Either::Right(values))
    };

    let (input, inner) = cut(context("ITERATE: issue in the content", inner_code))(input)?;

    let (input, _) = cut(context(
        "ITERATE: not closed",
        tuple((
            space0,
            alt((
                parse_directive_word("ENDITERATE"),
                parse_directive_word("ENDITER"),
                parse_directive_word("ENDI"),
                parse_directive_word("IEND")
            )),
            space0
        ))
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Iterate(counter, values, inner, iterate_start)
    ))
}

/// TODO
pub fn parse_basic(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let basic_start = input.clone();
    let (input, _) = tuple((space0, tag_no_case("LOCOMOTIVE"), space0))(input)?;

    let (input, args) = opt(separated_list1(
        preceded(space0, char(',')),
        preceded(space0, parse_label(false))
    ))(input)?;

    let (input, _) = tuple((space0, opt(tag("\r")), tag("\n")))(input)?;

    let (input, hidden_lines) =
        opt(terminated(preceded(space0, parse_basic_hide_lines), space0))(input)?;

    let (input, basic) = take_until("ENDLOCOMOTIVE")(input)?;

    let (input, _) = tuple((tag_no_case("ENDLOCOMOTIVE"), space0))(input)?;

    let args = args.map(|v| v.iter().map(|l| SmolStr::from(l.as_str())).collect_vec());

    let size = basic_start.input_len() - input.input_len();
    Ok((
        input,
        Token::Basic(args, hidden_lines, basic.to_string()).locate(basic_start, size)
    ))
}

/// Parse the instruction to hide basic lines
pub fn parse_basic_hide_lines(input: Z80Span) -> IResult<Z80Span, Vec<u16>, Z80ParserError> {
    let (input, _) = tuple((tag_no_case("HIDE_LINES"), space1))(input)?;
    separated_list1(
        preceded(space0, char(',')),
        preceded(space0, map(dec_number_inner, |d| d as u16))
    )(input)
}
#[inline]
pub fn dec_number_inner(input: Z80Span) -> IResult<Z80Span, u32, Z80ParserError> {
    let input_inner = input.deref().clone();
    let (input, number) = dec_number(input_inner).map_err(|_err| {
        cpclib_common::nom::Err::Error(
            VerboseError::from_error_kind(input, ErrorKind::AlphaNumeric).into()
        )
    })?;

    Ok((Z80Span(input), number))
}
#[inline]
pub fn bin_number_inner(input: Z80Span) -> IResult<Z80Span, u32, Z80ParserError> {
    let input_inner = input.deref().clone();
    let (input, number) = bin_number(input_inner).map_err(|_err| {
        cpclib_common::nom::Err::Error(
            VerboseError::from_error_kind(input, ErrorKind::AlphaNumeric).into()
        )
    })?;

    Ok((Z80Span(input), number))
}

#[inline]
pub fn hex_number_inner(input: Z80Span) -> IResult<Z80Span, u32, Z80ParserError> {
    let input_inner = input.deref().clone();
    let (input, number) = hex_number(input_inner).map_err(|_err| {
        cpclib_common::nom::Err::Error(
            VerboseError::from_error_kind(input, ErrorKind::AlphaNumeric).into()
        )
    })?;

    Ok((Z80Span(input), number))
}

pub fn parse_flag_value_inner(input: Z80Span) -> IResult<Z80Span, FlagValue, Z80ParserError> {
    let inner_input = input.deref().clone();

    let (input, number) = parse_flag_value(inner_input).map_err(|_err| {
        cpclib_common::nom::Err::Error(
            VerboseError::from_error_kind(input, ErrorKind::AlphaNumeric).into()
        )
    })?;

    Ok((Z80Span(input), number))
}

/// TODO - currently consume several lines. Should do it only one time
#[inline]
pub fn parse_empty_line(input: Z80Span) -> IResult<Z80Span, Option<LocatedToken>, Z80ParserError> {
    // let (input, _) = opt(line_ending)(input)?;
    let before_comment = input.clone();
    let (input, comment) = delimited(space0, opt(parse_comment), space0)(input)?;
    let (input, _) = alt((line_ending, eof))(input)?;

    let res = if comment.is_some() {
        let size = before_comment.input_len() - input.input_len();
        Some(comment.unwrap().locate(before_comment, size))
    }
    else {
        None
    };

    Ok((input, res))
}

#[inline]
fn parse_single_token(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    // Get the token
    let (input, opcode) = context(
        "[DBG] single token",
        preceded(
            space0,
            alt((
                context("[DBG] token", parse_token),
                context("[DBG] directive", parse_directive)
            ))
        )
    )(input)?;

    Ok((input, opcode))
}

fn eof(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    if input.len() == 0 {
        Ok((input.clone(), input))
    }
    else {
        Err(Err::Error(error_position!(input, ErrorKind::Eof)))
    }
}

#[derive(Clone, Copy, Debug)]
enum LabelModifier {
    Equ,
    Set,
    Equal(Option<BinaryOperation>),
    SetN,
    Next
}

#[inline]
pub fn parse_z80_directive_with_block(
    input: Z80Span
) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    alt((
        context("basic", parse_basic),
        context(
            "[DBG] block instruction",
            alt((
                context("[DBG] Macro definition", parse_macro),
                context("[DBG] crunched section", parse_crunched_section),
                context("[DBG] module", parse_module),
                context("[DBG] confined", parse_confined),
                context("[DBG] repeat", parse_repeat),
                context("[DBG] for", parse_for),
                context("[DBG] Function definition", parse_function),
                context("[DBG] SWITCH parse error", parse_switch),
                context("[DBG] iterate", parse_iterate),
                context("[DBG] while", parse_while),
                context("[DBG] rorg", parse_rorg),
                context("[DBG] condition", parse_conditional)
            ))
        )
    ))(input)
}

#[inline]
pub fn my_separated_list0<'vec, I, O, O2, E, F, G>(
    res: &'vec mut Vec<O>,
    mut sep: G,
    mut f: F
) -> impl FnMut(I) -> IResult<I, (), E> + 'vec
where
    I: Clone + InputLength,
    F: Parser<I, O, E> + 'vec,
    G: Parser<I, O2, E> + 'vec,
    E: ParseError<I>
{
    move |mut i: I| {
        match f.parse(i.clone()) {
            Err(Err::Error(_)) => return Ok((i, ())),
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                res.push(o);
                i = i1;
            }
        }

        loop {
            let len = i.input_len();
            match sep.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, ())),
                Err(e) => return Err(e),
                Ok((i1, _)) => {
                    // infinite loop check: the parser must always consume
                    if i1.input_len() == len {
                        return Err(Err::Error(E::from_error_kind(i1, ErrorKind::SeparatedList)));
                    }

                    match f.parse(i1.clone()) {
                        Err(Err::Error(_)) => return Ok((i, ())),
                        Err(e) => return Err(e),
                        Ok((i2, o)) => {
                            res.push(o);
                            i = i2;
                        }
                    }
                }
            }
        }
    }
}

/// Parse a line (ie a set of components separated by :) until the end of the line or a stop directive
/// XXX: In opposite to the other functions, the result is stored in the parameter (to avoid unecessary memory allocations)
#[inline]
pub fn parse_z80_line_complete(
    r#in: &mut Vec<LocatedToken>
) -> impl FnMut(Z80Span) -> IResult<Z80Span, (), Z80ParserError> + '_ {
    move |input: Z80Span| -> IResult<Z80Span, (), Z80ParserError> {
        // Early exit if line is empty
        let (input, empty) = opt(parse_empty_line)(input)?;
        if let Some(Some(notempty)) = empty {
            r#in.push(notempty);
            return Ok((input, ()));
        }

        // get the line components
        let (input, ()) = my_separated_list0_in(
            tuple((space0, tag(":"), space0)),
            // Take care of the order to not break parse
            alt((
                map(
                    // handle set/equ/ and so on
                    parse_z80_line_label_aware_directive,
                    |l| Either::Left(l)
                ),
                // move all of these in z80_line_component and rename line_component in something else ...
                map(
                    // a simple token mnemonic or directive (except macro call)
                    parse_single_token,
                    |t| Either::Left(t)
                ),
                map(
                    // macros/loops/...
                    preceded(space0, parse_z80_directive_with_block),
                    |b| Either::Left(b)
                ),
                map(
                    // a label followed by a simple token mnomonic or directive (except macro call)
                    pair(
                        terminated(parse_label(false), not(line_ending)),
                        parse_single_token
                    ),
                    |t| Either::Right(vec![LocatedToken::Label(t.0), t.1])
                ),
                // TODO add syntax where block-lmike directives have there name provided in a precedding label
                map(parse_macro_or_struct_call(false, false), |m| {
                    Either::Left(m)
                }),
                map(pair(space0, peek(tag(":"))), |_| Either::Right(vec![])), // a duplicated :
                map(preceded(space0, parse_label(false)), |l| {
                    Either::Left(LocatedToken::Label(l))
                })  // a single label
            )),
            r#in
        )(input)?;

        // we may have some space after the last component
        // also a : that is not cpatured when there is nothing after
        let (input, _) = tuple((space0, opt(tag(":")), space0))(input)?;

        // early stop in case of stop directive
        let (_, stop) = opt(parse_end_directive)(input.clone())?;
        if stop.is_some() {
            return Ok((input, ()));
        }

        // get the possible comment
        let (input, _) = space0(input)?;
        let before_comment = input.clone();
        let (input, comment) = opt(parse_comment)(input)?;
        let (input, _) = space0(input)?;

        if let Some(comment) = comment {
            let size = before_comment.input_len() - input.input_len();
            r#in.push(comment.locate(before_comment, size));
        }

        let (input, _) = cut(context(
            "Line ending expected",
            preceded(
                opt(char(':')), // we allow : as the very last char of a line
                alt((eof, line_ending))
            )
        ))(input)?;

        Ok((input, ()))
    }
}

#[inline]
pub fn parse_assign_operator(
    input: Z80Span
) -> IResult<Z80Span, Option<BinaryOperation>, Z80ParserError> {
    let (rest, word) = is_a("=<>+-*/%^|&")(input.clone())?;
    let oper = match word.as_str() {
        "=" => None,

        ">>=" => Some(BinaryOperation::RightShift),
        "<<=" => Some(BinaryOperation::LeftShift),

        "+=" => Some(BinaryOperation::Add),
        "-=" => Some(BinaryOperation::Sub),
        "*=" => Some(BinaryOperation::Mul),
        "/=" => Some(BinaryOperation::Div),
        "%=" => Some(BinaryOperation::Mod),

        "&=" => Some(BinaryOperation::BinaryAnd),
        "|=" => Some(BinaryOperation::BinaryOr),
        "^=" => Some(BinaryOperation::BinaryXor),

        "&&=" => Some(BinaryOperation::BooleanAnd),
        "||=" => Some(BinaryOperation::BooleanOr),

        _ => {
            return Err(Err::Error(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Alt
            )))
        }
    };

    Ok((rest, oper))
}

/// No opcodes are expected there.
/// Initially it was supposed to manage lines with only labels, however it has been extended
/// to labels fallowed by specific commands.
/// TODO this complete piece of code MUST be removed and integrated within parse_z80_line_complete
#[inline]
pub fn parse_z80_line_label_aware_directive(
    input: Z80Span
) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let before_label = input.clone();

    let (input, r#let) = opt(delimited(space0, parse_directive_word("LET"), space0))(input)?;

    let _after_let = input.clone();
    let (input, label) = context("Label issue", preceded(space0, parse_label(true)))(input)?; // here there is true because of arkos tracker 2 player

    let (next, label_modifier) =
        opt(preceded(space0, is_a("DEFLdeflQUquSTNstnXx=<>+-*/%^|&")))(input.clone())?;

    let (input, label_modifier): (Z80Span, Option<LabelModifier>) = match label_modifier {
        Some(label_modifier) => {
            let mut label_modifier =
                smartstring::SmartString::<smartstring::Compact>::from(label_modifier.as_str());
            label_modifier.as_mut_str().make_ascii_uppercase();

            match label_modifier.as_str() {
                "DEFL" => (next, Some(LabelModifier::Equ)),
                "EQU" => (next, Some(LabelModifier::Equ)),
                "SETN" => (next, Some(LabelModifier::SetN)),
                "NEXT" => (next, Some(LabelModifier::Next)),
                "SET" => {
                    if tuple((space0, expr, parse_comma))(next.clone()).is_err() {
                        (next, Some(LabelModifier::Set))
                    }
                    else {
                        (input, None)
                    }
                }
                "=" => (next, Some(LabelModifier::Equal(None))),
                oper => {
                    let oper = match oper {
                        ">>=" => Some(BinaryOperation::RightShift),
                        "<<=" => Some(BinaryOperation::LeftShift),

                        "+=" => Some(BinaryOperation::Add),
                        "-=" => Some(BinaryOperation::Sub),
                        "*=" => Some(BinaryOperation::Mul),
                        "/=" => Some(BinaryOperation::Div),
                        "%=" => Some(BinaryOperation::Mod),

                        "&=" => Some(BinaryOperation::BinaryAnd),
                        "|=" => Some(BinaryOperation::BinaryOr),
                        "^=" => Some(BinaryOperation::BinaryXor),

                        "&&=" => Some(BinaryOperation::BooleanAnd),
                        "||=" => Some(BinaryOperation::BooleanOr),

                        _ => None
                    };

                    if oper.is_some() {
                        (next, Some(LabelModifier::Equal(oper)))
                    }
                    else {
                        (input, None)
                    }
                }
            }
        }

        None => (input, None)
    };

    // early quite if there is only one label and nothing else
    if let Option::None = label_modifier {
        if r#let.is_some() {
            return Err(cpclib_common::nom::Err::Failure(
                VerboseError::from_error_kind(before_label, ErrorKind::Char).into()
            ));
        }
        else {
            // ensure there is nothing after
            let _ = alt((
                tuple((my_space0, tag(":"))),
                tuple((my_space0, my_line_ending))
            ))(input.clone())?;
            return Ok((input, LocatedToken::Label(label)));
        }
    }

    let label_modifier = label_modifier.unwrap();

    // ensure let uses =
    if r#let.is_some() {
        if let LabelModifier::Equal(..) = &label_modifier {
            // ok
        }
        else {
            return Err(cpclib_common::nom::Err::Failure(
                VerboseError::from_error_kind(before_label, ErrorKind::Char).into()
            ));
        }
    }

    let (input, expr_arg) = match &label_modifier {
        LabelModifier::Equ | LabelModifier::Equal(..) | LabelModifier::Set => {
            cut(context("Value error", map(expr, |e| Some(e))))(input)?
        }
        _ => (input, None)
    };

    let (input, source_label) = match &label_modifier {
        LabelModifier::Next | LabelModifier::SetN => {
            cut(context(
                "Label expected",
                map(preceded(space0, parse_label(false)), |l| Some(l))
            ))(input)?
        }
        _ => (input, None)
    };

    // optional expression to control the displacement
    let (input, additional_arg) = match &label_modifier {
        LabelModifier::Next | LabelModifier::SetN => opt(preceded(parse_comma, expr))(input)?,
        _ => (input, None)
    };

    // opt!(char!(':')) >>

    // Build the needed token for the label of interest
    let token = match label_modifier {
        LabelModifier::Equ => Token::Equ(label.into(), expr_arg.unwrap()),
        LabelModifier::Equal(op) => Token::Assign(label.into(), expr_arg.unwrap(), op),
        LabelModifier::Set => Token::Assign(label.into(), expr_arg.unwrap(), None),
        LabelModifier::SetN => {
            Token::SetN(label.into(), source_label.unwrap().into(), additional_arg)
        }
        LabelModifier::Next => {
            Token::Next(label.into(), source_label.unwrap().into(), additional_arg)
        }
    };

    // add it to the list
    let size = before_label.input_len() - input.input_len();

    Ok((input, token.locate(before_label, size)))
}
pub fn parse_fname(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    parse_string(input)
}

/// Parser for file names in appropriate directives
pub fn parse_string(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    alt((
    preceded(tag("\""), terminated(take_until("\""), take(1usize))),
    verify(
        preceded(tag("'"), terminated(take_until("'"), take(1usize))),
        |s: &Z80Span| s.len() > 1
    ),
    )) // single quote is stricly reserved for chars now, so we accept strings with 2 chars at minimum
    (input)
}

pub fn parse_charset(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, charset) = opt(alt((parse_charset_string, parse_charset_start_stop_end)))(input)?;

    Ok((
        input,
        charset
            .map(|c| Token::Charset(c))
            .unwrap_or_else(|| Token::Charset(CharsetFormat::Reset))
    ))
}

pub fn parse_charset_start_stop_end(
    input: Z80Span
) -> IResult<Z80Span, CharsetFormat, Z80ParserError> {
    let (input, (start, stop, end)) = tuple((
        expr,
        preceded(parse_comma, expr),
        opt(preceded(parse_comma, expr))
    ))(input)?;

    let format = if let Some(end) = end {
        CharsetFormat::Interval(start, stop, end)
    }
    else {
        CharsetFormat::Char(start, stop)
    };
    Ok((input, format))
}

pub fn parse_charset_string(input: Z80Span) -> IResult<Z80Span, CharsetFormat, Z80ParserError> {
    // manage the string format - TODO manage the others too
    let (input, chars) = context("Missing string", parse_string)(input)?;
    let (input, start) = context("Missing start value", preceded(parse_comma, expr))(input)?;
    let format = CharsetFormat::CharsList(chars.chars().collect_vec(), start);

    Ok((input, format))
}

/// Parser for the include directive
pub fn parse_include(
    include_start: Z80Span
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        let (input, once_fname) = pair(
            opt(delimited(space0, parse_word("ONCE"), space0)),
            parse_fname
        )(input)?;

        let (once, fname) = once_fname;

        let (input, namespace) = opt(preceded(
            delimited(
                space0,
                alt((
                    tag_no_case("namespace"),
                    tag_no_case("module"),
                    tag_no_case("as")
                )),
                space0
            ),
            delimited(
                tag("\""),
                parse_label(false),
                tag("\"") // TODO modify to accept only labels without dot
            )
        ))(input)?;

        let size = include_start.len() - input.len();
        Ok((
            input,
            LocatedToken::Include(fname, namespace, once.is_some(), include_start.take(size))
        ))
    }
}

/// Parse for the various binary include directives
#[inline]
pub fn parse_incbin(
    input_start: Z80Span,
    transformation: BinaryTransformation
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        let (input, fname) = preceded(space0, parse_fname)(input)?;

        let (input, offset) =
            opt(preceded(tuple((space0, char(','), space0)), located_expr))(input)?;
        let (input, length) =
            opt(preceded(tuple((space0, char(','), space0)), located_expr))(input)?;
        let (input, _extended_offset) =
            opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
        let (input, off) = opt(preceded(
            tuple((space0, char(','), space0)),
            tag_no_case("OFF")
        ))(input)?;

        let span = input_start.take(input_start.input_len() - input.input_len());
        Ok((
            input,
            LocatedToken::Incbin {
                fname: fname,
                offset,
                length,
                extended_offset: None,
                off: off.is_some(),
                transformation,
                span
            }
        ))
    }
}

/// parse write direct in memory / converted to a bank directive
/// we do not care of the parameters for roms as we are not working in an emulator
pub fn parse_write_direct_memory(
    input_start: Z80Span
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        // filter all the stuff before
        let (input, _) = tuple((
            tag_no_case("DIRECT"),
            space1,
            tag_no_case("-1"),
            parse_comma,
            tag_no_case("-1"),
            parse_comma
        ))(input)?;

        let (input, bank) = expr(input)?;

        let token = Token::Bank(Some(bank));
        let size = input_start.input_len() - input.input_len();
        let token = token.locate(input_start.clone(), size);

        Ok((
            input,
            LocatedToken::WarningWrapper(
                Box::new(token),
                "Prefer BANK or PAGE directives to write direct -1, -1, XX".to_owned()
            )
        ))
    }
}

#[derive(PartialEq)]
pub enum SaveKind {
    Save,
    WriteDirect
}

/// Parse both save directive and write direct in a file
pub fn parse_save(
    input_start: Z80Span,
    save_kind: SaveKind
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        let input = if save_kind == SaveKind::WriteDirect {
            parse_word("DIRECT")(input)?.0
        }
        else {
            input
        };

        let (input, filename) = parse_fname(input)?;

        let (input, address) = opt(preceded(parse_comma, opt(expr)))(input)?;
        let (input, size) = if address.is_some() {
            opt(preceded(parse_comma, opt(expr)))(input)?
        }
        else {
            (input, None)
        };

        let (input, save_type) = if size.is_some() && save_kind == SaveKind::Save {
            opt(preceded(
                parse_comma,
                alt((
                    value(SaveType::AmsdosBin, parse_word("AMSDOS")),
                    value(SaveType::AmsdosBas, parse_word("BASIC")),
                    value(SaveType::Dsk, parse_word("DSK")),
                    value(SaveType::Tape, parse_word("TAPE"))
                ))
            ))(input)?
        }
        else {
            if save_kind == SaveKind::WriteDirect {
                (input, Some(SaveType::AmsdosBin))
            }
            else {
                (input, None)
            }
        };

        let (input, dsk_filename) = if save_type.is_some() && save_kind == SaveKind::Save {
            opt(preceded(parse_comma, parse_fname))(input)?
        }
        else {
            (input, None)
        };

        let (input, side) = if dsk_filename.is_some() && save_kind == SaveKind::Save {
            opt(preceded(parse_comma, expr))(input)?
        }
        else {
            (input, None)
        };

        let filename = filename.to_string();
        let dsk_filename = dsk_filename.map(|s| s.to_string());

        let span_size = input_start.input_len() - input.input_len();

        Ok((
            input,
            Token::Save {
                filename,
                address: address.unwrap_or(None),
                size: size.unwrap_or(None),
                save_type,
                dsk_filename,
                side
            }
            .locate(input_start.clone(), span_size)
        ))
    }
}

/// Parse  UNDEF directive.
pub fn parse_undef(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, label) = parse_label(false)(input)?;

    Ok((input, Token::Undef(label.into())))
}

/// Parse return from a function
pub fn parse_return(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, expr) = expr(input)?;

    Ok((input, Token::Return(expr)))
}

pub fn parse_section(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, name) = preceded(space0, parse_label(false))(input)?;

    Ok((input, Token::Section(name.into())))
}

pub fn parse_range(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, start) = cut(context(
        "RANGE: wrong start address",
        delimited(space0, expr, space0)
    ))(input)?;
    let (input, stop) = cut(context(
        "RANGE: wrong end address",
        preceded(parse_comma, delimited(space0, expr, space0))
    ))(input)?;
    let (input, label) = cut(context(
        "RANGE: wrong name",
        preceded(parse_comma, delimited(space0, parse_label(false), space0))
    ))(input)?;

    Ok((input, Token::Range(label.as_str().to_owned(), start, stop)))
}

pub fn parse_assign(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, (label, op, value)) = tuple((
        parse_label(false),
        delimited(space0, parse_assign_operator, space0),
        expr
    ))(input)?;

    Ok((input, Token::Assign(label.into(), value, op)))
}

pub fn parse_token(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let parsing_state = input.context().state.clone();

    verify(alt((parse_token1, parse_token2)), move |t| {
        t.is_accepted(&parsing_state)
    })(input)
}

pub fn parse_token1(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    parse_opcode_no_arg(input)
}

pub fn parse_token2(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let input_start = input.clone();

    // Get the first word that will drive the rest of parsing
    let (rest, word) = delimited(space0, alpha1, space0)(input.clone())?;

    let mut word: smartstring::SmartString<smartstring::Compact> =
        smartstring::SmartString::from(word.as_str());
    word.as_mut_str().make_ascii_uppercase();

    // Apply the right parsing
    // We use this way of doing to reduce function calls and error. Let's hope it will speed everything
    match word.as_str() {
        // located tokens
        "LD" => parse_ld(input_start)(rest),

        // tokens to locate
        word => {
            let (input, token) = match word {
                "ADC" => parse_add_or_adc(Mnemonic::Adc)(rest),
                "ADD" => parse_add_or_adc(Mnemonic::Add)(rest),
                "AND" => parse_logical_operator(Mnemonic::And)(rest),

                "BIT" => parse_res_set_bit(Mnemonic::Bit)(rest),

                "CALL" => parse_call_jp_or_jr(Mnemonic::Call)(rest),
                "CP" => parse_cp(rest),

                "DEC" => parse_inc_dec(Mnemonic::Dec)(rest),
                "DJNZ" => parse_djnz(rest),

                "EX" => alt((parse_ex_af, parse_ex_hl_de, parse_ex_mem_sp))(rest),

                "EXA" => Ok((rest, Token::new_opcode(Mnemonic::ExAf, None, None))),
                "EXD" => Ok((rest, Token::new_opcode(Mnemonic::ExHlDe, None, None))),

                "IN" => parse_in(rest),
                "INC" => parse_inc_dec(Mnemonic::Inc)(rest),
                "IM" => parse_im(rest),

                "JP" => parse_call_jp_or_jr(Mnemonic::Jp)(rest),
                "JR" => parse_call_jp_or_jr(Mnemonic::Jr)(rest),

                "OR" => parse_logical_operator(Mnemonic::Or)(rest),
                "OUT" => parse_out(rest),

                "POP" => parse_push_n_pop(Mnemonic::Pop)(rest),
                "PUSH" => parse_push_n_pop(Mnemonic::Push)(rest),

                "RES" => parse_res_set_bit(Mnemonic::Res)(rest),
                "RET" => parse_ret(rest),
                "RLC" => parse_shifts_and_rotations(Mnemonic::Rlc)(rest),
                "RL" => parse_shifts_and_rotations(Mnemonic::Rl)(rest),
                "RRC" => parse_shifts_and_rotations(Mnemonic::Rrc)(rest),
                "RR" => parse_shifts_and_rotations(Mnemonic::Rr)(rest),
                "RST" => parse_rst(rest),

                "SBC" => parse_sbc(rest),
                "SET" => parse_res_set_bit(Mnemonic::Set)(rest),
                "SL1" => parse_shifts_and_rotations(Mnemonic::Sl1)(rest),
                "SLA" => parse_shifts_and_rotations(Mnemonic::Sla)(rest),
                "SLL" => parse_shifts_and_rotations(Mnemonic::Sl1)(rest),
                "SRA" => parse_shifts_and_rotations(Mnemonic::Sra)(rest),
                "SRL" => parse_shifts_and_rotations(Mnemonic::Srl)(rest),
                "SUB" => parse_sub(rest),

                "XOR" => parse_logical_operator(Mnemonic::Xor)(rest),

                _ => {
                    Err(Err::Error(Z80ParserError::from_error_kind(
                        input,
                        ErrorKind::Alt
                    )))
                }
            }?;

            let size = input_start.input_len() - input.input_len();
            Ok((input, token.locate(input_start, size)))
        }
    }
}

/// Parse ex af, af' instruction
pub fn parse_ex_af(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(
        value(
            (),
            tuple((
                //        parse_word("EX"),
                parse_register_af,
                parse_comma,
                parse_word("AF'")
            ))
        ),
        |_| Token::new_opcode(Mnemonic::ExAf, None, None)
    )(input)
}

/// Parse ex hl, de instruction
pub fn parse_ex_hl_de(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(
        alt((
            value(
                (),
                tuple((
                    //          tag_no_case("EX"),
                    //          space1,
                    parse_register_hl,
                    parse_comma,
                    parse_register_de
                ))
            ),
            value(
                (),
                tuple((
                    //            tag_no_case("EX"),
                    //        space1,
                    parse_register_de,
                    parse_comma,
                    parse_register_hl
                ))
            )
        )),
        |_| Token::new_opcode(Mnemonic::ExHlDe, None, None)
    )(input)
}

/// Parse ex (sp), hl
pub fn parse_ex_mem_sp(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, destination) = tuple((
        //     tag_no_case("EX"),
        //      space1,
        char('('),
        space0,
        parse_register_sp,
        space0,
        char(')'),
        parse_comma,
        alt((parse_register_hl, parse_indexregister16))
    ))(input)?;

    Ok((
        input,
        Token::new_opcode(Mnemonic::ExMemSp, Some(destination.6), None)
    ))
}

/// Parse any directive
pub fn parse_directive(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let parsing_state = input.context().state.clone();
    verify(parse_directive_new, move |d| d.is_accepted(&parsing_state))(input.clone())
}

#[inline]
pub fn parse_directive_new(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let input_start = input.clone();

    // Get the first word that will drive the rest of parsing
    let (rest, word) =
        delimited(space0, terminated(alphanumeric1, not(is_a("._"))), space0)(input.clone())?;

    let mut upper_word: smartstring::SmartString<smartstring::Compact> =
        smartstring::SmartString::from(word.as_str());
    upper_word.as_mut_str().make_ascii_uppercase();

    match upper_word.as_str() {
        "DB" | "DEFB" | "DM" | "DEFM" | "BYTE" | "TEXT" => {
            parse_db_or_dw_or_str(input_start, 0)(rest)
        }
        "WORD" | "DW" | "DEFW" => parse_db_or_dw_or_str(input_start, 1)(rest),
        "STR" => parse_db_or_dw_or_str(input_start, 2)(rest),

        "INCBIN" | "BINCLUDE" => parse_incbin(input_start, BinaryTransformation::None)(rest),
        "INCEXO" => {
            parse_incbin(input_start, BinaryTransformation::Crunch(CrunchType::LZEXO))(rest)
        }
        "INCLZ4" => parse_incbin(input_start, BinaryTransformation::Crunch(CrunchType::LZ4))(rest),
        "INCL48" => parse_incbin(input_start, BinaryTransformation::Crunch(CrunchType::LZ48))(rest),
        "INCL49" => parse_incbin(input_start, BinaryTransformation::Crunch(CrunchType::LZ49))(rest),
        "INCAPU" => {
            parse_incbin(input_start, BinaryTransformation::Crunch(CrunchType::LZAPU))(rest)
        }
        "INCZX0" => parse_incbin(input_start, BinaryTransformation::Crunch(CrunchType::LZX0))(rest),

        "INCLUDE" | "READ" => parse_include(input_start)(rest),

        "STRUCT" => parse_struct(input_start)(rest),
        "SAVE" => parse_save(input_start, SaveKind::Save)(rest),
        "WRITE" => {
            alt((
                parse_save(input_start.clone(), SaveKind::WriteDirect),
                parse_write_direct_memory(input_start.clone())
            ))(rest)
        }

        word => {
            let (input, token) = match word {
                "FILL" | "DS" | "DEFS" | "RMEM" => parse_defs(rest),

                "ALIGN" => parse_align(rest),
                "ASSERT" => parse_assert(rest),

                "BANK" => parse_bank(rest),
                "BANKSET" => parse_bankset(rest),
                "BREAKPOINT" => parse_breakpoint(rest),
                "BUILDSNA" => parse_buildsna(rest),

                "CHARSET" => parse_charset(rest),

                "DEFSECTION" => parse_range(rest),

                "END" => Ok((rest, Token::End)),
                "EXPORT" => parse_export(ExportKind::Export)(rest),

                "FAIL" => parse_fail(rest),

                "LIMIT" => parse_limit(rest),
                "LIST" => Ok((rest, Token::List)),

                "NOEXPORT" => parse_export(ExportKind::NoExport)(rest),
                "NOLIST" => Ok((rest, Token::NoList)),
                "NOP" => parse_nop(rest),

                "ORG" => parse_org(rest),

                "PAUSE" => Ok((rest, Token::Pause)),
                "PRINT" => parse_print(rest),
                "PROTECT" => parse_protect(rest),

                "RANGE" => parse_range(rest),
                "RETURN" => parse_return(rest),
                "RUN" => parse_run(rest),

                "SECTION" => parse_section(rest),
                "SNASET" => parse_snaset(rest),

                "SNAPINIT" | "SNAINIT" => parse_snainit(rest),

                "TICKER" => parse_stable_ticker(rest),

                "UNDEF" => parse_undef(rest),

                "WAITNOPS" => parse_waitnops(rest),

                _ => {
                    Err(Err::Error(Z80ParserError::from_error_kind(
                        input,
                        ErrorKind::Alt
                    )))
                }
            }?;

            let size = input_start.input_len() - input.input_len();
            Ok((input, token.locate(input_start, size)))
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum KindOfConditional {
    If,
    IfNot,
    IfDef,
    IfNdef,
    IfUsed,
    IfNused
}

/// Parse if expression.TODO finish the implementation in order to have ELSEIF and ELSE branches"
/// TODO shorten the string code source
pub fn parse_conditional(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let if_start = input.clone();

    let mut conditions = Vec::new();
    let mut else_clause = None;

    let mut input_loop = input.clone();
    loop {
        let first_loop = conditions.is_empty();

        // Gest the kind of test to do - it can fail after an else
        let if_token_or_error = alt((
            map(parse_directive_word("IF"), |_| KindOfConditional::If),
            map(parse_directive_word("IFNOT"), |_| KindOfConditional::IfNot),
            map(parse_directive_word("IFDEF"), |_| KindOfConditional::IfDef),
            map(parse_directive_word("IFNDEF"), |_| {
                KindOfConditional::IfNdef
            }),
            map(parse_directive_word("IFUSED"), |_| {
                KindOfConditional::IfUsed
            }),
            map(parse_directive_word("IFNUSED"), |_| {
                KindOfConditional::IfNused
            })
        ))(input_loop.clone());

        if first_loop && if_token_or_error.is_err() {
            return Err(if_token_or_error.err().unwrap());
        }

        let (input, condition) = if let Ok((input, test_kind)) = if_token_or_error {
            // Get the corresponding test
            let (input, cond) = cut(context(
                "Condition: error in the condition",
                delimited(space0, parse_conditional_condition(test_kind), space0)
            ))(input)?;
            (input, Some(cond))
        }
        else {
            (input_loop, None)
        };

        let (input, _) = cut(context(
            "Condition: condition must end by a new line or ':'",
            alt((
                recognize(delimited(space0, parse_comment, line_ending)),
                line_ending,
                tag(":")
            ))
        ))(input)?;

        let _code_input = input.clone();
        let (input, code) = cut(context(
            "Condition: syntax error in conditionnal code",
            inner_code
        ))(input)?;

        if let Some(condition) = condition {
            conditions.push((condition, code));

            let (input, r#else) = opt(preceded(
                my_many0_nocollect(alt((space1, line_ending, tag(":")))),
                parse_directive_word("ELSE")
            ))(input)?;
            input_loop = input;
            if r#else.is_none() {
                break;
            }
        }
        else {
            else_clause = Some(code);
            input_loop = input;
            break;
        }
    }

    let (input, _) = context(
        "Condition: issue in end condition",
        tuple((
            opt(alt((
                delimited(space0, tag(":"), space0),
                recognize(delimited(space0, parse_comment, line_ending))
            ))),
            recognize(cut(preceded(space0, parse_directive_word("ENDIF"))))
        ))
    )(input_loop)?;

    Ok((input, LocatedToken::If(conditions, else_clause, if_start)))
}

/// Read the condition part in the parse_conditional macro
#[inline]
fn parse_conditional_condition(
    code: KindOfConditional
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedTestKind, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedTestKind, Z80ParserError> {
        match &code {
            KindOfConditional::If => map(located_expr, |e| LocatedTestKind::True(e))(input),

            KindOfConditional::IfNot => map(located_expr, |e| LocatedTestKind::False(e))(input),

            KindOfConditional::IfDef => {
                map(preceded(space0, parse_label(false)), |l| {
                    LocatedTestKind::LabelExists(l)
                })(input)
            }

            KindOfConditional::IfNdef => {
                map(parse_label(false), |l| {
                    LocatedTestKind::LabelDoesNotExist(l)
                })(input)
            }

            KindOfConditional::IfUsed => {
                map(parse_label(false), |l| LocatedTestKind::LabelUsed(l))(input)
            }

            KindOfConditional::IfNused => {
                map(parse_label(false), |l| LocatedTestKind::LabelNused(l))(input)
            }

            _ => unreachable!()
        }
    }
}

/// Parse a breakpint instruction
pub fn parse_breakpoint(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(opt(expr), |exp| Token::Breakpoint(exp))(input)
}

pub fn parse_bankset(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, count) = expr(input)?;

    Ok((input, Token::Bankset(count)))
}

pub fn parse_buildsna(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    terminated(
        map(
            cut(opt(alt((tag_no_case("V2"), tag_no_case("V3"))))),
            |v: Option<Z80Span>| {
                Token::BuildSna(match v {
                    Some(txt) => {
                        if txt.to_lowercase() == "v2" {
                            Some(SnapshotVersion::V2)
                        }
                        else {
                            Some(SnapshotVersion::V3)
                        }
                    }
                    None => None
                })
            }
        ),
        not(alphanumeric1)
    )(input)
}

pub fn parse_run(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, exp) = cut(context("RUN expects an expression (e.g. RUN $)", expr))(input)?;
    let (input, ga) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;

    Ok((input, Token::Run(exp, ga)))
}

pub fn parse_limit(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, exp) = expr(input)?;

    Ok((input, Token::Limit(exp)))
}

pub fn parse_waitnops(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, exp) = expr(input)?;

    Ok((input, Token::WaitNops(exp)))
}

/// Parse tickin directives
pub fn parse_stable_ticker(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    alt((parse_stable_ticker_start, parse_stable_ticker_stop))(input)
}

/// Parse begining of ticker
pub fn parse_stable_ticker_start(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(
        preceded(tuple((tag_no_case("start"), space1)), parse_label(false)),
        |name| Token::StableTicker(StableTickerAction::Start(name.into()))
    )(input)
}

/// Parse end of ticker
pub fn parse_stable_ticker_stop(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(tag_no_case("stop"), |_| {
        Token::StableTicker(StableTickerAction::Stop)
    })(input)
}

pub fn parse_bank(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, count) = opt(expr)(input)?;

    Ok((input, Token::Bank(count)))
}

/// Parse fake and real LD instructions
pub fn parse_ld(
    input_start: Z80Span
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        context(
            "[DBG] ld",
            alt((
                context("[DBG] fake ld", parse_ld_fake(input_start.clone())),
                context("[DBG] normal ld", parse_ld_normal(input_start.clone()))
            ))
        )(input)
    }
}

/// Parse artifical LD instruction (would be replaced by several real instructions)
pub fn parse_ld_fake(
    input_start: Z80Span
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        // let (input, _) = tuple((tag_no_case("LD"), space1))(input)?;

        let (input, dst) = alt((
            terminated(
                alt((parse_register16, parse_indexregister16)),
                not(alt((tag_no_case(".low"), tag_no_case(".high"))))
            ),
            parse_hl_address,
            parse_indexregister_with_index
        ))(input)?;

        let (input, _) = parse_comma(input)?;

        // TODO - add https://z00m128.github.io/sjasmplus/documentation.html#s_fake_instructions
        let (input, src) = if dst.is_register16() {
            alt((
                terminated(
                    alt((parse_register16, parse_indexregister16)),
                    not(alt((tag_no_case(".low"), tag_no_case(".high"))))
                ),
                parse_hl_address,
                parse_indexregister_with_index
            ))(input)?
        }
        else
        // mem-like
        {
            terminated(
                parse_register16,
                not(alt((tag_no_case(".low"), tag_no_case(".high"))))
            )(input)?
        };

        let token = Token::new_opcode(Mnemonic::Ld, Some(dst), Some(src));
        let size = input_start.input_len() - input.input_len();
        let token = token.locate(input_start.clone(), size);

        let warning = LocatedToken::WarningWrapper(
            Box::new(token),
            "This is a fake instruction assembled using several opcodes".into()
        );

        Ok((input, warning))
    }
}

/// Parse the valids LD versions
pub fn parse_ld_normal(
    input_start: Z80Span
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        //  let (input, _) = context("[DBG] ...", tuple((space0, parse_word("LD"), space0)))(input)?;

        let _start = input.clone();
        let (input, dst) = cut(context(
            LD_WRONG_DESTINATION,
            alt((
                parse_reg_address,
                parse_indexregister_address,
                parse_indexregister_with_index,
                parse_register_sp,
                terminated(
                    parse_register16,
                    not(alt((tag_no_case(".low"), tag_no_case(".high"))))
                ),
                parse_register8,
                parse_indexregister16,
                parse_indexregister8,
                parse_register_i,
                parse_register_r,
                parse_hl_address,
                parse_address
            ))
        ))(input)?;

        let (input, _) = context("LD: missing comma", cut(parse_comma))(input)?;

        // src possibilities depend on dst
        let (input, src) = cut(context(LD_WRONG_SOURCE, cut(parse_ld_normal_src(&dst))))(input)?;

        let token = Token::new_opcode(Mnemonic::Ld, Some(dst), Some(src));
        let size = input_start.input_len() - input.input_len();
        let token = token.locate(input_start.clone(), size);

        Ok((input, token))
    }
}

/// Parse the source of LD depending on its destination
#[inline]
fn parse_ld_normal_src(
    dst: &DataAccess
) -> impl Fn(Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> + '_ {
    move |input: Z80Span| {
        if dst.is_register_sp() {
            alt((
                parse_register_hl,
                parse_indexregister16,
                parse_address,
                parse_expr
            ))(input)
        }
        else if dst.is_address_in_register16() || dst.is_address_in_indexregister16() {
            // by construction is t is HL/IX/IY
            alt((parse_register8, parse_expr))(input)
        }
        else if dst.is_register16() | dst.is_indexregister16() {
            alt((parse_address, parse_expr))(input)
        }
        else if dst.is_register8() {
            // todo find a way to merge them together
            if dst.is_register_a() {
                alt((
                    parse_indexregister_with_index,
                    parse_reg_address,
                    parse_indexregister_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8,
                    parse_register_i,
                    parse_register_r,
                    parse_expr
                ))(input)
            }
            else {
                alt((
                    parse_indexregister_address,
                    parse_indexregister_with_index,
                    parse_hl_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8,
                    parse_expr
                ))(input)
            }
        }
        else if dst.is_indexregister8() {
            alt((
                parse_indexregister_address,
                parse_indexregister_with_index,
                parse_hl_address,
                parse_address,
                parse_register8,
                verify(alt((parse_register_ixh, parse_register_ixl)), |_| {
                    dst.is_register_ixl() || dst.is_register_ixh()
                }),
                verify(alt((parse_register_iyh, parse_register_iyl)), |_| {
                    dst.is_register_iyl() || dst.is_register_iyh()
                }),
                parse_expr
            ))(input)
        }
        else if dst.is_memory() {
            alt((
                parse_register16,
                parse_register8,
                parse_register_sp,
                parse_indexregister16
            ))(input)
        }
        else if dst.is_address_in_register16() {
            parse_register8(input)
        }
        else if dst.is_indexregister_with_index() {
            alt((parse_register8, parse_expr))(input)
        }
        else if dst.is_register_i() || dst.is_register_r() {
            parse_register_a(input)
        }
        else {
            Err(cpclib_common::nom::Err::Error(
                VerboseError::from_error_kind(input, ErrorKind::Alt).into()
            ))
        }
    }
}

/// Parse RES, SET and BIT instructions
#[inline]
pub fn parse_res_set_bit(
    res_or_set: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, bit) = cut(context("Wrong bit definition", parse_expr))(input)?;

        let (input, _) = cut(parse_comma)(input)?;

        let (input, operand) = cut(context(
            "Wrong destination",
            alt((
                parse_register8,
                parse_hl_address,
                parse_indexregister_with_index
            ))
        ))(input)?;

        // Bit and Res can copy the result in a reg
        let (input, hidden_arg) = if res_or_set == Mnemonic::Bit {
            (input, None)
        }
        else {
            opt(preceded(parse_comma, parse_register8))(input)?
        };

        Ok((
            input,
            Token::OpCode(
                res_or_set,
                Some(bit),
                Some(operand),
                hidden_arg.map(|d| d.get_register8().unwrap())
            )
        ))
    }
}

/// Parse CP tokens
pub fn parse_cp(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(
        //   preceded(
        //    parse_word("CP"),
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr
        )), //   )
        |operand| Token::new_opcode(Mnemonic::Cp, Some(operand), None)
    )(input)
}

#[derive(PartialEq)]
pub enum ExportKind {
    Export,
    NoExport
}

#[inline]
pub fn parse_export(
    code: ExportKind
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    #[inline]
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, labels) = cut(context(
            "Wrong parameters",
            separated_list0(parse_comma, parse_label(false))
        ))(input)?;

        let labels = labels.iter().map(|l| SmolStr::from(l.as_str())).collect_vec(); // TODO really use LocatedToken
        if code == ExportKind::Export {
            Ok((input, Token::Export(labels)))
        }
        else {
            Ok((input, Token::NoExport(labels)))
        }
    }
}

#[inline]
/// Parse DB DW directives
pub fn parse_db_or_dw_or_str(
    input_start: Z80Span,
    code: u8
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        let (input, expr) = expr_list(input)?;

        let token_span = input_start.take(input_start.input_len() - input.input_len()); // TODO Use a real type that embeds strings in a Z80Span to avoid copying them

        Ok((
            input,
            if code == 0 {
                LocatedToken::Defb(expr, token_span)
            }
            else if code == 1 {
                LocatedToken::Defw(expr, token_span)
            }
            else
            // if code == 2
            {
                LocatedToken::Str(expr, token_span)
            }
        ))
    }
}

// Fail if we do not read a forbidden keyword
pub fn parse_forbidden_keyword(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    let (input, _) = space0(input)?;
    let (input, name) = context(
        "Unable to read directive name",
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")
    )(input)?;

    let mut end_directive_iter = if input.context().options().dotted_directive {
        DOTTED_END_DIRECTIVE.iter()
    }
    else {
        END_DIRECTIVE.iter()
    };
    if !end_directive_iter
        .find(|&&a| a == name.to_ascii_uppercase())
        .is_some()
    {
        return Err(Err::Error(
            VerboseError::from_error_kind(name, ErrorKind::AlphaNumeric).into()
        ));
    }

    let (input, _) = space0(input)?;

    Ok((input, name))
}
pub fn parse_macro_arg(input: Z80Span) -> IResult<Z80Span, LocatedMacroParam, Z80ParserError> {
    let _start_input = input.clone();
    let (stop_input, param) = alt((
        map(
            delimited(
                tuple((space0, char('['))),
                separated_list0(char(','), parse_macro_arg),
                tuple((char(']'), space0))
            ),
            |l| {
                LocatedMacroParam::List(
                    l.into_iter()
                        .map(|p| Box::new(p.clone()))
                        .collect::<Vec<_>>()
                )
            }
        ),
        map(
            delimited(
                space0,
                alt((
                    recognize(expr), // TODO handle evaluation or transposition
                    string_between_quotes,
                    recognize(my_many0_nocollect(none_of(" ,\r\n\t][;:")))
                )), // TODO find a way to give arguments with space
                alt((space0, eof))
            ),
            |s| LocatedMacroParam::Single(s)
        )
    ))(input)?;

    Ok((stop_input, param))
}

/// Manage the call of a macro.
/// When ambiguou may return a label
#[inline]
pub fn parse_macro_or_struct_call(
    allowed_to_return_a_label: bool,
    for_struct: bool
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input| {
        // BUG: added because of parsing issues. Need to find why and remove ot
        let (input_label, _) = space0(input)?;
        let input_start = input_label.clone();
        let (input, name) = terminated(
            parse_macro_name,
            not(alt((
                recognize(pair(space0, alt((tag(":"), line_ending, eof)))),
                recognize(char('.'))
            )))
        )(input_label.clone())?;

        // Check if the macro name is allowed
        if !allowed_label(
            &name.to_ascii_uppercase(),
            input.context().options().dotted_directive
        ) {
            return Err(Err::Failure(
                cpclib_common::nom::error::VerboseError::<Z80Span>::add_context(
                    input_label,
                    if for_struct {
                        "STRUCT: forbidden name"
                    }
                    else {
                        "MACRO or STRUCT: forbidden name"
                    },
                    cpclib_common::nom::error::ParseError::<Z80Span>::from_error_kind(
                        input,
                        ErrorKind::AlphaNumeric
                    )
                )
                .into()
            ));
        }

        let nothing_after =
            pair(space0, alt((recognize(parse_comment), tag(":"), tag("\n"))))(input.clone())
                .is_ok();

        if allowed_to_return_a_label && nothing_after {
            let token = LocatedToken::Label(name.clone());
            let msg = format!("Ambiguous code. Use (void) for macro with no args, (default) for struct with default parameters; avoid labels that do not start at beginning of a line. {} is considered to be a label, not a macro.", name);
            let warning = LocatedToken::WarningWrapper(Box::new(token), msg);
            return Ok((input, warning));
        }

        let (input, _) = pair(space0, not(parse_comment))(input)?;

        let (input, args) = if alt((eof, tag("\n"), tag(":")))(input.clone()).is_ok() {
            (input, vec![])
        }
        else {
            cut(context(
                if for_struct {
                    "STRUCT: error in arguments list"
                }
                else {
                    "MACRO or STRUCT: forbidden name"
                },
                alt((
                    map(delimited(space0, tag_no_case("(void)"), space0), |_| {
                        Default::default()
                    }),
                    alt((
                        map(tag_no_case("(void)"), |_| Vec::new()),
                        separated_list1(
                            parse_comma,
                            alt((
                                parse_macro_arg,
                                map(space1, |space: Z80Span| {
                                    LocatedMacroParam::Single(space.take(0))
                                    // string of size 0;
                                })
                            ))
                        )
                    ))
                ))
            ))(input.clone())?
        };

        if args.len() == 1 && args.first().unwrap().is_empty() {
            panic!();
        }

        // avoid ambiguate code such as label nop
        if args.len() == 1 {
            let arg = &args[0];
            let arg = arg.span();
            if alt((parse_word("NOP"), recognize(parse_opcode_no_arg)))(arg).is_ok() {
                return Err(Err::Failure(
                    (cpclib_common::nom::error::VerboseError::<Z80Span>::add_context(
                        input_label,
                        if for_struct {
                            "First argument of STRUCT cannot be an opcode with no argument"
                        }
                        else {
                            "First argument of MACRO or STRUCT cannot be an opcode with no argument"
                        },
                        cpclib_common::nom::error::ParseError::<Z80Span>::from_error_kind(
                            input,
                            ErrorKind::AlphaNumeric
                        )
                    ))
                    .into()
                ));
            }
        }

        let all_span = input_start.take(input_start.input_len() - input.input_len());
        Ok((input, LocatedToken::MacroCall(name, args, all_span)))
    }
}

#[inline]
fn parse_directive_word(
    name: &'static str
) -> impl Fn(Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> + 'static {
    move |input: Z80Span| {
        if input.context().options().dotted_directive {
            preceded(tag("."), parse_word(name))(input)
        }
        else {
            parse_word(name)(input)
        }
    }
}

#[inline]
fn parse_word(name: &'static str) -> impl Fn(Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    #[inline]
    move |input: Z80Span| {
        map(
            tuple((
                tag_no_case(name),
                alt((
                    map(eof, |_| ()),
                    map(
                        pair(
                            not(one_of(
                                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"
                            )),
                            space0
                        ),
                        |_| ()
                    )
                ))
            )),
            |v| v.0
        )(input)
    }
}

/// ...
pub fn parse_djnz(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(preceded(opt(parse_comma), parse_expr), |expr| {
        Token::new_opcode(Mnemonic::Djnz, Some(expr), None)
    })(input)
}

/// ...
pub fn expr_list(input: Z80Span) -> IResult<Z80Span, Vec<LocatedExpr>, Z80ParserError> {
    separated_list1(
        tuple((tag(","), space0)),
        cut(context(
            "Error in expression",
            alt((string_expr, located_expr))
        ))
    )(input)
}

/// ...
pub fn parse_assert(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, expr) = cut(context("ASSERT: expression error", expr))(input)?;

    let (input, exps) = cut(context(
        "ASSERT: comment error",
        opt(preceded(parse_comma, parse_print_inner))
    ))(input)?;

    Ok((input, Token::Assert(expr, exps)))
}

/// ...
pub fn parse_align(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, boundary) = expr(input)?;
    let (input, fill) = opt(preceded(parse_comma, expr))(input)?;

    Ok((input, Token::Align(boundary, fill)))
}

pub fn parse_print_inner(input: Z80Span) -> IResult<Z80Span, Vec<FormattedExpr>, Z80ParserError> {
    separated_list1(
        parse_comma,
        alt((
            formatted_expr,
            map(expr, FormattedExpr::from),
            map(string_between_quotes, {
                |s: Z80Span| {
                    FormattedExpr::from(Expr::String(SmolStr::from_iter(s.fragment().chars())))
                }
            })
        ))
    )(input)
}
/// ...
pub fn parse_print(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(cut(parse_print_inner), |exps| Token::Print(exps))(input)
}

pub fn parse_fail(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(opt(parse_print_inner), |exps| Token::Fail(exps))(input)
}

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
fn formatted_expr(input: Z80Span) -> IResult<Z80Span, FormattedExpr, Z80ParserError> {
    let (input, _) = char('{')(input)?;
    let (input, format) = alt((
        map(tag_no_case("INT"), |_| ExprFormat::Int),
        map(tag_no_case("HEX4"), |_| ExprFormat::Hex(Some(4))),
        map(tag_no_case("HEX8"), |_| ExprFormat::Hex(Some(8))),
        map(tag_no_case("HEX2"), |_| ExprFormat::Hex(Some(2))),
        map(tag_no_case("HEX"), |_| ExprFormat::Hex(None)),
        map(tag_no_case("BIN8"), |_| ExprFormat::Bin(Some(8))),
        map(tag_no_case("BIN16"), |_| ExprFormat::Bin(Some(16))),
        map(tag_no_case("BIN32"), |_| ExprFormat::Bin(Some(32))),
        map(tag_no_case("BIN"), |_| ExprFormat::Bin(None))
    ))(input)?;
    let (input, _) = char('}')(input)?;

    let (input, _) = space0(input)?;

    let (input, exp) = expr(input)?;

    Ok((input, FormattedExpr::Formatted(format, exp)))
}

/// Handle \ in end of line
#[inline]
fn my_space0(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    recognize(opt(my_space1))(input)
}

/// Handle \ in end of line
#[inline]
fn my_space1(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    recognize(my_many1_nocollect(alt((
        map(eof, |_| ()),
        map(
            tuple((
                space0,
                tag("\\"), // do we keep it ?
                opt(pair(space0, parse_comment)),
                line_ending,
                space0
            )),
            |_| ()
        ),
        map(space1, |_| ())
    ))))(input)
}

#[inline]
fn my_line_ending(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    alt((line_ending, tag(":")))(input)
}

#[inline]
fn parse_comma(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    delimited(my_space0, tag(","), my_space0)(input)
}

/// ...
pub fn parse_protect(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, start) = expr(input)?;

    let (input, end) = preceded(parse_comma, expr)(input)?;

    Ok((input, Token::Protect(start, end)))
}

#[inline]
/// ...
pub fn parse_logical_operator(
    operator: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, operand) = context(
            "Wrong logical operand",
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
        )(input)?;

        Ok((input, Token::new_opcode(operator, Some(operand), None)))
    }
}

/// Substraction with A register
pub fn parse_sub(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    //  let (input, _) = tag_no_case("SUB")(input)?;
    //  let (input, _) = space1(input)?;
    let (input, operand) = alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr
    ))(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Sub, Some(operand), None)))
}

/// Par se the SBC instruction
pub fn parse_sbc(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    //  let (input, _) = tag_no_case("SBC")(input)?;
    //   let (input, _) = space1(input)?;

    let (input, opera) = opt(terminated(
        alt((parse_register_a, parse_register_hl)),
        parse_comma
    ))(input)?;

    let opera = opera.unwrap_or(DataAccess::Register8(Register8::A));

    let (input, operb) = if opera.is_register_a() {
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr
        ))(input)
    }
    else {
        alt((parse_register16, parse_register_sp))(input)
    }?;

    Ok((
        input,
        Token::new_opcode(Mnemonic::Sbc, Some(opera), Some(operb))
    ))
}

/// Parse ADC and ADD instructions
#[inline]
pub fn parse_add_or_adc(
    add_or_adc: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, first) = opt(terminated(
            alt((
                map(parse_register_a, |_| DataAccess::Register8(Register8::A)),
                map(parse_register_hl, |_| {
                    DataAccess::Register16(Register16::Hl)
                }),
                parse_indexregister16
            )),
            parse_comma
        ))(input)?;

        // no operand implies it is A
        let first = first.unwrap_or(DataAccess::Register8(Register8::A));

        let (input, second) = if first.is_register8() {
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))(input)
        }
        else if first.is_register16() {
            alt((parse_register16, parse_register_sp))(input) // Case for HL XXX AF is accepted whereas it is not the case in real life
        }
        else if first.is_indexregister16() {
            alt((
                parse_register_bc,
                parse_register_de,
                parse_register_hl,
                parse_register_sp,
                verify(parse_register_ix, |_| first.is_register_ix()),
                verify(parse_register_iy, |_| first.is_register_iy())
            ))(input)
        }
        else {
            return Err(cpclib_common::nom::Err::Error(
                VerboseError::from_error_kind(input, ErrorKind::Alt).into()
            ));
        }?;

        Ok((
            input,
            Token::new_opcode(add_or_adc, Some(first), Some(second))
        ))
    }
}

/// ...
#[inline]
pub fn parse_push_n_pop(
    push_or_pop: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, registers) =
            separated_list1(parse_comma, alt((parse_register16, parse_indexregister16)))(input)?;

        if registers.len() > 1 {
            match push_or_pop {
                Mnemonic::Push => Ok((input, Token::MultiPush(registers))),
                Mnemonic::Pop => Ok((input, Token::MultiPop(registers))),
                _ => unreachable!()
            }
        }
        else {
            Ok((
                input,
                Token::new_opcode(push_or_pop, Some(registers[0].clone()), None)
            ))
        }
    }
}

/// ...
pub fn parse_ret(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(opt(parse_flag_test), |cond| {
        Token::new_opcode(
            Mnemonic::Ret,
            if cond.is_some() {
                Some(DataAccess::FlagTest(cond.unwrap()))
            }
            else {
                None
            },
            None
        )
    })(input)
}

/// ...
#[inline]
pub fn parse_inc_dec(
    inc_or_dec: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, register) = alt((
            parse_register16,
            parse_indexregister16,
            parse_register8,
            parse_indexregister8,
            parse_register_sp,
            parse_hl_address,
            parse_indexregister_with_index
        ))(input)?;

        Ok((input, Token::new_opcode(inc_or_dec, Some(register), None)))
    }
}

/// TODO manage other out formats
pub fn parse_out(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    //  let (input, _) = parse_word("OUT")(input)?;

    // get the port proposal
    let (input, port) = alt((parse_portc, parse_portnn))(input)?;

    // the vlaue depends on the port
    let (input, value) = if port.is_portc() {
        // reg c
        opt(preceded(
            parse_comma,
            alt((
                parse_register8,
                map(alt((parse_word("f"), tag("0"))), |_| {
                    DataAccess::from(Expr::from(0))
                })
            ))
        ))(input)?
    }
    else {
        map(preceded(parse_comma, parse_register_a), |reg| Some(reg))(input)?
    };
    let value = value.unwrap_or(DataAccess::from(Expr::from(0)));

    Ok((
        input,
        Token::new_opcode(Mnemonic::Out, Some(port), Some(value))
    ))
}

/// Parse all the in flavors
pub fn parse_in(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    // let (input, _) = parse_word("IN")(input)?;
    let zero = DataAccess::from(Expr::from(0));

    // get the port proposal
    let (input, destination) = opt(terminated(
        alt((
            parse_register8,
            value(zero.clone(), alt((tag_no_case("f"), tag("0"))))
        )),
        parse_comma
    ))(input)?;
    let destination = destination.unwrap_or(zero);

    let (input, port) = cut(alt((
        parse_portc,
        verify(parse_portnn, |_| {
            destination.get_register8().unwrap().is_a()
        })
    )))(input)?;

    Ok((
        input,
        Token::new_opcode(Mnemonic::In, Some(destination), Some(port))
    ))
}

/// Parse the rst instruction
pub fn parse_rst(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    // let (input, _) = parse_word("RST")(input)?;
    let (input, val) = parse_expr(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Rst, Some(val), None)))
}

/// Parse the IM instruction
pub fn parse_im(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    // let (input, _) = parse_word("IM")(input)?;
    let (input, val) = parse_expr(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Im, Some(val), None)))
}

/// Parse all RLC, RL, RR, SLA, SRA flavors
/// RLC A
/// RLC B
/// RLC C
/// RLC D
/// RLC E
/// RLC H
/// RLC L
/// RLC (HL)
/// RLC (IX+n)
/// RLC (IY+n)
#[inline]
pub fn parse_shifts_and_rotations(
    oper: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    #[inline]
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, arg) = alt((
            parse_register8,
            parse_hl_address,
            parse_indexregister_with_index
        ))(input)?;

        // hidden opcodes
        let (input, arg2) = opt(preceded(parse_comma, parse_register8))(input)?;

        Ok((input, Token::new_opcode(oper, Some(arg), arg2)))
    }
}

/// TODO reduce the flag space for jr"],
#[inline]
pub fn parse_call_jp_or_jr(
    call_jp_or_jr: Mnemonic
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    #[inline]
    move |input: Z80Span| -> IResult<Z80Span, Token, Z80ParserError> {
        let (input, flag_test) = opt(terminated(parse_flag_test, parse_comma))(input)?;

        let (input, dst) = cut(context(
            match call_jp_or_jr {
                Mnemonic::Jp => JP_WRONG_PARAM,
                Mnemonic::Jr => JR_WRONG_PARAM,
                Mnemonic::Call => CALL_WRONG_PARAM,
                _ => unreachable!()
            },
            alt((
                verify(
                    alt((
                        parse_hl_address,
                        parse_indexregister_address,
                        parse_register_hl,
                        parse_indexregister16
                    )),
                    |_| call_jp_or_jr.is_jp() && flag_test.is_none()
                ), // not possible for call and for jp/jr when there is flag
                parse_expr
            ))
        ))(input)?;

        // Allow to parse JP HL as to be JP (HL) original notation is misleading
        let dst = match dst {
            DataAccess::IndexRegister16(reg) => DataAccess::MemoryIndexRegister16(reg),
            DataAccess::Register16(reg) => DataAccess::MemoryRegister16(reg),
            other => other
        };

        let flag_test = if flag_test.is_some() {
            Some(DataAccess::FlagTest(flag_test.unwrap()))
        }
        else {
            None
        };

        Ok((
            input,
            Token::new_opcode(call_jp_or_jr, flag_test, Some(dst))
        ))
    }
}

/// ...
pub fn parse_flag_test(input: Z80Span) -> IResult<Z80Span, FlagTest, Z80ParserError> {
    alt((
        map(parse_word("NZ"), |_| FlagTest::NZ),
        map(parse_word("Z"), |_| FlagTest::Z),
        map(parse_word("NC"), |_| FlagTest::NC),
        map(parse_word("C"), |_| FlagTest::C),
        map(parse_word("PO"), |_| FlagTest::PO),
        map(parse_word("PE"), |_| FlagTest::PE),
        map(parse_word("P"), |_| FlagTest::P),
        map(parse_word("M"), |_| FlagTest::M)
    ))(input)
}

// XXX to remove as soon as possible
// named_attr!(#[doc="TODO"],
// parse_dollar <&str, Expr>, do_parse!(
// tag!("$") >>
// (Expr::Label(String::from("$")))
// )
// );

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
pub fn parse_register16(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    let (next, code) = recognize(terminated(
        pair(one_of("abdhABDH"), one_of("fcelFCEL")),
        not(alpha1)
    ))(input.clone())?;

    let code = code.to_ascii_uppercase();
    let reg = match code.as_str() {
        "AF" => DataAccess::Register16(Register16::Af),
        "BC" => DataAccess::Register16(Register16::Bc),
        "DE" => DataAccess::Register16(Register16::De),
        "HL" => DataAccess::Register16(Register16::Hl),
        _ => {
            return Err(Err::Error(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Alt
            )))
        }
    };

    Ok((next, reg))
}

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
pub fn parse_register8(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    alt((
        map(
            tuple((
                parse_register16,
                preceded(
                    tag("."),
                    alt((
                        value('L', tag_no_case("low")),
                        value('H', tag_no_case("high"))
                    ))
                ),
                space0
            )),
            |(r16, code, _)| {
                if code == 'L' {
                    r16.to_data_access_for_low_register().unwrap()
                }
                else {
                    r16.to_data_access_for_high_register().unwrap()
                }
            }
        ),
        parse_register_a,
        parse_register_b,
        parse_register_c,
        parse_register_d,
        parse_register_e,
        parse_register_h,
        parse_register_l
    ))(input)
}

/// Parse register i
pub fn parse_register_i(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    value(
        DataAccess::SpecialRegisterI,
        tuple((tag_no_case("I"), not(alphanumeric1)))
    )(input)
}

/// Parse register r
pub fn parse_register_r(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    value(
        DataAccess::SpecialRegisterR,
        tuple((tag_no_case("R"), not(alphanumeric1)))
    )(input)
}

macro_rules! parse_any_register8 {
    ($name: ident, $char:expr, $reg:expr) => {
        /// Parse register $char
        pub fn $name(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
            value(DataAccess::Register8($reg), parse_word($char))(input)
        }
    };
}

parse_any_register8!(parse_register_a, "A", Register8::A);
parse_any_register8!(parse_register_b, "B", Register8::B);
parse_any_register8!(parse_register_c, "C", Register8::C);
parse_any_register8!(parse_register_d, "d", Register8::D);
parse_any_register8!(parse_register_e, "e", Register8::E);
parse_any_register8!(parse_register_h, "h", Register8::H);
parse_any_register8!(parse_register_l, "l", Register8::L);

/// Produce the function that parse a given register
#[inline]
fn register16_parser(
    representation: &'static str,
    register: Register16
) -> impl for<'src, 'ctx> Fn(Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    #[inline]
    move |input: Z80Span| {
        value(
            DataAccess::Register16(register),
            tuple((tag_no_case(representation), not(alphanumeric1)))
        )(input)
    }
}

macro_rules! parse_any_register16 {
    ($name: ident, $char:expr, $reg:expr) => {
        /// Parse the $char register and return it as a DataAccess
        pub fn $name(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
            register16_parser($char, $reg)(input)
        }
    };
}

parse_any_register16!(parse_register_sp, "SP", Register16::Sp);
parse_any_register16!(parse_register_af, "AF", Register16::Af);
parse_any_register16!(parse_register_bc, "BC", Register16::Bc);
parse_any_register16!(parse_register_de, "DE", Register16::De);
parse_any_register16!(parse_register_hl, "HL", Register16::Hl);

/// Parse the IX register
pub fn parse_register_ix(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    value(
        DataAccess::IndexRegister16(IndexRegister16::Ix),
        tuple((tag_no_case("IX"), not(alphanumeric1)))
    )(input)
}

/// Parse the IY register
pub fn parse_register_iy(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    value(
        DataAccess::IndexRegister16(IndexRegister16::Iy),
        tuple((tag_no_case("IY"), not(alphanumeric1)))
    )(input)
}

// TODO find a way to not use that
macro_rules! parse_any_indexregister8 {
    ($($reg:ident, $alias1:ident, $alias2:ident)*) => {$(
        paste::paste! {
            /// Parse register $reg
            pub fn [<parse_register_ $reg:lower>] (input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
                value(
                    DataAccess::IndexRegister8(IndexRegister8::$reg),
                    tuple((
                        alt((
                            parse_word( stringify!($reg)),
                            parse_word( stringify!($alias1)),
                            parse_word( stringify!($alias2)),
                        ))
                        , not(alphanumeric1)))
                    )(input)
                }
            }
        )*}
    }
parse_any_indexregister8!(
    Ixh,hx,xh
    Ixl,lx,xl
    Iyh,hy,yh
    Iyl,ly,yl
);

/// Parse and indexed register in 8bits
pub fn parse_indexregister8(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    alt((
        parse_register_ixh,
        parse_register_iyh,
        parse_register_ixl,
        parse_register_iyl
    ))(input)
}

/// Parse a 16 bits indexed register
pub fn parse_indexregister16(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    terminated(
        map(
            alt((
                map(tag_no_case("IX"), |_| IndexRegister16::Ix),
                map(tag_no_case("IY"), |_| IndexRegister16::Iy)
            )),
            |reg| DataAccess::IndexRegister16(reg)
        ),
        not(alphanumeric1)
    )(input)
}

/// Parse the use of an indexed register as (IX + 5)"
pub fn parse_indexregister_with_index(
    input: Z80Span
) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    let (input, reg) = preceded(tuple((tag("("), space0)), parse_indexregister16)(input)?;

    let (input, op) = preceded(
        space0,
        alt((
            value(BinaryOperation::Add, tag("+")),
            value(BinaryOperation::Sub, tag("-"))
        ))
    )(input)?;

    let (input, expr) = terminated(expr, tuple((space0, tag(")"))))(input)?;

    Ok((
        input,
        DataAccess::IndexRegister16WithIndex(
            reg.get_indexregister16().unwrap(),
            match op {
                BinaryOperation::Add => expr,
                BinaryOperation::Sub => expr.neg(),
                _ => unreachable!()
            }
        )
    ))
}

/// Parse (C) used in in/out
pub fn parse_portc(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    value(
        DataAccess::PortC,
        tuple((tag("("), space0, parse_register_c, space0, tag(")")))
    )(input)
}

/// Parse (nn) used in in/out
pub fn parse_portnn(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    map(
        delimited(tag("("), expr, preceded(space0, tag(")"))),
        |address| DataAccess::PortN(address)
    )(input)
}

/// Parse an address access `(expression)`
pub fn parse_address(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    map(
        delimited(
            tag("("),
            expr,
            terminated(
                preceded(space0, tag(")")),
                not(
                    // filter expressions ; they are followed by some operators
                    preceded(space0, is_a("/+=-*<>%"))
                )
            )
        ),
        |address| DataAccess::Memory(address)
    )(input)
}

/// Parse (R16)
pub fn parse_reg_address(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    map(
        delimited(
            terminated(tag("("), space0),
            parse_register16,
            preceded(space0, tag(")"))
        ),
        |reg| DataAccess::MemoryRegister16(reg.get_register16().unwrap())
    )(input)
}

/// Parse (HL)
pub fn parse_hl_address(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    value(
        DataAccess::MemoryRegister16(Register16::Hl),
        delimited(
            terminated(tag("("), space0),
            parse_register_hl,
            preceded(space0, tag(")"))
        )
    )(input)
}

/// Parse (ix) and (iy)
pub fn parse_indexregister_address(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    map(
        delimited(
            terminated(tag("("), space0),
            parse_indexregister16,
            preceded(space0, tag(")"))
        ),
        |reg| DataAccess::MemoryIndexRegister16(reg.get_indexregister16().unwrap())
    )(input)
}

/// Parse an expression and returns it inside a DataAccession::Expression
pub fn parse_expr(input: Z80Span) -> IResult<Z80Span, DataAccess, Z80ParserError> {
    let (input, expr) = expr(input)?;
    Ok((input, DataAccess::Expression(expr)))
}

/// Parse standard org directive
pub fn parse_org(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, val1) = cut(context("Invalid argument", expr))(input)?;
    let (input, val2) = opt(preceded(parse_comma, expr))(input)?;

    Ok((input, Token::Org(val1, val2)))
}

/// Parse defs instruction. TODO add optional parameters
pub fn parse_defs(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, val) = separated_list1(
        parse_comma,
        cut(context(
            "Wrong argument",
            tuple((expr, opt(preceded(parse_comma, expr))))
        ))
    )(input)?;

    Ok((input, Token::Defs(val)))
}

pub fn parse_nop(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, val) = cut(context(
        "Wrong argument. NOP expects an expression",
        opt(expr)
    ))(input)?;

    Ok((
        input,
        Token::OpCode(Mnemonic::Nop, val.map(|e| e.into()), None, None)
    ))
}

/// Parse any opcode having no argument
pub fn parse_opcode_no_arg(input: Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    let (input, token) = map(
        preceded(
            space0,
            consumed(map_opt(alpha1, |word: Z80Span| {
                match word.as_str().to_ascii_uppercase().as_str() {
                    "CCF" => Some(Mnemonic::Ccf),
                    "CPD" => Some(Mnemonic::Cpd),
                    "CPDR" => Some(Mnemonic::Cpdr),
                    "CPI" => Some(Mnemonic::Cpi),
                    "CPIR" => Some(Mnemonic::Cpir),
                    "CPL" => Some(Mnemonic::Cpl),
                    "DAA" => Some(Mnemonic::Daa),
                    "DI" => Some(Mnemonic::Di),
                    "EI" => Some(Mnemonic::Ei),
                    "EXX" => Some(Mnemonic::Exx),
                    "HALT" => Some(Mnemonic::Halt),
                    "IND" => Some(Mnemonic::Ind),
                    "INDR" => Some(Mnemonic::Indr),
                    "INI" => Some(Mnemonic::Ini),
                    "INIR" => Some(Mnemonic::Inir),
                    "LDD" => Some(Mnemonic::Ldd),
                    "LDDR" => Some(Mnemonic::Lddr),
                    "LDI" => Some(Mnemonic::Ldi),
                    "LDIR" => Some(Mnemonic::Ldir),
                    "NEG" => Some(Mnemonic::Neg),
                    "NOPS2" => Some(Mnemonic::Nop2),
                    "OTDR" => Some(Mnemonic::Otdr),
                    "OTIR" => Some(Mnemonic::Otir),
                    "OUTD" => Some(Mnemonic::Outd),
                    "OUTDR" => Some(Mnemonic::Otdr),
                    "OUTI" => Some(Mnemonic::Outi),
                    "OUTIR" => Some(Mnemonic::Otir),
                    "RETI" => Some(Mnemonic::Reti),
                    "RETN" => Some(Mnemonic::Retn),
                    "RLA" => Some(Mnemonic::Rla),
                    "RLCA" => Some(Mnemonic::Rlca),
                    "RLD" => Some(Mnemonic::Rld),
                    "RRA" => Some(Mnemonic::Rra),
                    "RRCA" => Some(Mnemonic::Rrca),
                    "RRD" => Some(Mnemonic::Rrd),
                    "SCF" => Some(Mnemonic::Scf),
                    _ => None
                }
            }))
        ),
        |(span, mne)| {
            LocatedToken::Standard {
                token: Token::OpCode(mne, None, None, None),
                span
            }
        }
    )(input)?;

    Ok((input, token))
}

fn parse_snainit(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, fname) = parse_fname(input)?;

    Ok((input, Token::SnaInit(fname.to_string())))
}

fn parse_struct(
    input_start: Z80Span
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    move |input: Z80Span| -> IResult<Z80Span, LocatedToken, Z80ParserError> {
        let (input, name) = cut(parse_label(false))(input)?;

        // TODO parse inner with filtering on the allowed operations
        // would be easier to write and would allow conditional operations
        let (input, fields) = cut(context(
            "STRUCT: error in inner content",
            many1(delimited(
                my_many0_nocollect(alt((
                    space1,
                    recognize(parse_comment),
                    line_ending,
                    tag(":")
                ))),
                pair(
                    context(
                        "STRUCT: label error",
                        verify(terminated(parse_label(false), space1), |label: &Z80Span| {
                            label.to_ascii_lowercase() != "endstruct"
                        })
                    ),
                    cut(context(
                        "STRUCT: Invalid operation",
                        verify(
                            alt((parse_directive, parse_macro_or_struct_call(false, true))),
                            |t| {
                                true | t.is_call_macro_or_build_struct()
                                    | t.is_db()
                                    | t.is_dw()
                                    | t.is_str()
                            }
                        )
                    ))
                ),
                my_many0_nocollect(alt((
                    space1,
                    recognize(parse_comment),
                    line_ending,
                    tag(":")
                )))
            ))
        ))(input)?;

        let (input, _) = cut(preceded(space0, parse_directive_word("ENDSTRUCT")))(input)?;

        let all_span = input_start.take(input_start.input_len() - input.input_len());
        Ok((input, LocatedToken::Struct(name, fields, all_span)))
    }
}

fn parse_snaset(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    let (input, flagname) = cut(context(SNASET_WRONG_LABEL, parse_label(false)))(input)?;
    let (input, _) = context(SNASET_MISSING_COMMA, cut(parse_comma))(input)?;

    let (input, values) = cut(separated_list1(
        delimited(space0, parse_comma, space0),
        parse_flag_value_inner
    ))(input)?;

    let flagname = SmolStr::from(flagname.as_str());

    let (flagname, value) = if values.len() == 1 {
        (flagname, values[0].clone())
    }
    else {
        (
            format!("{}:{}", flagname, values[0].as_u16().unwrap()).into(),
            values[1].clone()
        )
    };

    let (_, flag) = (parse_flag(LocatedSpan::new(&flagname))).map_err(|_e| {
        cpclib_common::nom::Err::Error(
            VerboseError::from_error_kind(input.clone(), ErrorKind::AlphaNumeric).into()
        )
    })?;
    Ok((input, Token::SnaSet(flag, value)))
}

/// Parse a comment that start by `;` and ends at the end of the line.
pub fn parse_comment(input: Z80Span) -> IResult<Z80Span, Token, Z80ParserError> {
    map(
        preceded(alt((tag(";"), tag("//"))), take_till(|ch| ch == '\n')),
        |string: Z80Span| Token::Comment(string.to_string())
    )(input)
}

/// Usefull later for db
pub fn string_between_quotes(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    delimited(char('\"'), is_not("\""), char('\"'))(input)
}

/// TODO
pub fn string_expr(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    map(string_between_quotes, |string| LocatedExpr::String(string))(input)
}

pub fn char_expr(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, c) = alt((
        delimited(
            tag("\""),
            cpclib_common::nom::character::complete::anychar,
            tag("\"")
        ),
        delimited(
            tag("'"),
            cpclib_common::nom::character::complete::anychar,
            tag("'")
        )
    ))(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, LocatedExpr::Char(c, span)))
}

/// Parse a label(label: S)
/// TODO reimplement to never build a string
#[inline]
pub fn parse_label(
    doubledots: bool
) -> impl Fn(Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    #[inline]
    move |input: Z80Span| {
        let _start = input.clone();


        // Finger crosses that no allocation is done there
        let (input, obtained_label) = recognize(tuple((
            opt(alt((tag("::"), tag("@"), tag(".")))),
            alt((
                recognize(one_of(
                    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"
                )),
                recognize(delimited(char('{'), expr, char('}')))
            )),
            my_many0_nocollect(alt((
                is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"),
                tag("."),
                recognize(delimited(char('{'), expr, char('}')))
            )))
        )))(input)?;

/*
        // fail to parse a label when it is 100% sure it corresponds to  a macro call
        let (input, macro_arg) = opt(preceded(space1, tag_no_case("(void)".into())))(input)?;
        if macro_arg.is_some() {
            return Err(cpclib_common::nom::Err::Error(error_position!(
                input,
                ErrorKind::OneOf
            )));
        }
*/
        let start_with_double_dots = obtained_label.len() > 2 && &obtained_label[..2] == "::";
        let true_label = if start_with_double_dots {
            &obtained_label[2..]
        }
        else {
            &obtained_label[..]
        };

        //needed because of AT2
        let input = if doubledots {
            let (input, _) = opt(tag_no_case(":"))(input)?;
            input
        }
        else {
            input
        };
        

        // Be sure that ::ld is not considered to be a label
        let label_len = true_label.len();
        if label_len >= MIN_MAX_LABEL_SIZE.0 &&
        label_len <= DOTTED_MIN_MAX_LABEL_SIZE.1 &&
            !allowed_label( &true_label.to_ascii_uppercase(), input.context().options().dotted_directive)  {
            Err(cpclib_common::nom::Err::Error(error_position!(
                input,
                ErrorKind::OneOf
            )))
        }
        else {
            Ok((input, obtained_label))
        }
    }
}

#[inline]
fn impossible_names(dotted_directive: bool) -> &'static [&'static str] {
    if dotted_directive {
        &DOTTED_IMPOSSIBLE_NAMES
    }
    else {
        &IMPOSSIBLE_NAMES
    }
}

#[inline]
fn allowed_label(name: &str, dotted_directive: bool) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    let iter = impossible_names(dotted_directive).par_iter();
    #[cfg(target_arch = "wasm32")]
    let mut iter = impossible_names(dotted_directive).iter();

    !iter.any(|&content| content == name)
}

pub fn parse_end_directive(input: Z80Span) -> IResult<Z80Span, String, Z80ParserError> {
    let (input, dot) = if input.context().options().dotted_directive {
        value(Some('.'), tag("."))(input)?
    }
    else {
        (input, None)
    };

    let (input, keyword) =
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")(input)?;
    let keyword = dot
        .as_ref()
        .into_iter()
        .cloned()
        .chain(keyword.iter_elements().into_iter())
        .collect::<String>()
        .to_ascii_uppercase();

    let mut end_directive_iter = if input.context().options().dotted_directive {
        DOTTED_END_DIRECTIVE.iter()
    }
    else {
        END_DIRECTIVE.iter()
    };
    if end_directive_iter.any(|&val| val == &keyword) {
        Ok((input, keyword))
    }
    else {
        Err(cpclib_common::nom::Err::Error(error_position!(
            input,
            ErrorKind::OneOf
        )))
    }
}

pub fn parse_macro_name(input: Z80Span) -> IResult<Z80Span, Z80Span, Z80ParserError> {
    let dotted_directive = input.context().options().dotted_directive;
    verify(
        recognize(tuple((
            one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"),
            is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"),
            not(char('{'))
        ))),
        move |name: &Z80Span| {
            let _first = name.fragment().chars().next().unwrap();
            let keyword = name.as_str().to_ascii_uppercase();

            if !allowed_label(&keyword, dotted_directive) {
                return false;
            }
            else {
                return true;
            }
        }
    )(input)
}

pub fn prefixed_label_expr(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();
    let (input, _) = space0(input_start.clone())?;

    let (input, prefix) = alt((
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}")),
        value(LabelPrefix::Pageset, tag_no_case("{pageset}"))
    ))(input)?;

    let (input, label) = preceded(space0, alt((parse_label(false), tag("$$"), tag("$"))))(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, LocatedExpr::PrefixedLabel(prefix, label, span)))
}

// Parse an ASM file an returns the stream of tokens.
// pub fn parse_file(fname: String) -> Vec<Token> {
// let mut f = File::open(fnmae).expect(format!("{} not found", fname));
// let mut contents = String::new();
// f.read_to_string(&mut contents)
// .expect(format!("Something went wrong reading {}", fname));
//
//
// parse_binary_stream(fname.to_bytes())
// }

// XXX Code greatly inspired from https://github.com/Geal/nom/blob/master/tests/arithmetic_ast.rs

/// Read a value
pub fn parse_value(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let alien_start = input.deref().clone();
    let (alien_end, val) =
        alt((hex_number, dec_number, bin_number))(alien_start.clone()).map_err(|_op| {
            cpclib_common::nom::Err::Error(
                VerboseError::from_error_kind(input, ErrorKind::Verify).into()
            )
        })?;

    let (span, input) = input_start.take_split(alien_start.input_len() - alien_end.input_len());
    Ok((input, LocatedExpr::Value(val as i32, span)))
}

/// Parse a repetition counter
pub fn parse_counter(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    map(
        recognize(delimited(
            tag("{".into()),
            parse_label(false), // BUG will accept too many cases
            pair(tag("}".into()), not(alphanumeric1))
        )),
        |l| LocatedExpr::Label(l)
    )(input)
}

/// Read a parenthesed expression
pub fn parens(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, exp) = delimited(
        delimited(my_space0, tag("("), my_space0),
        located_expr,
        delimited(my_space0, tag(")"), space0)
    )(input)?;

    let span = input_start.take(input_start.len() - input.len());
    Ok((input, LocatedExpr::Paren(Box::new(exp), span)))
}

pub fn parse_expr_bracketed_list(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, list) = delimited(
        pair(tag("["), my_space0),
        separated_list0(parse_comma, located_expr),
        pair(my_space0, tag("]"))
    )(input)?;

    let span = input_start.take(input_start.len() - input.len());
    Ok((input, LocatedExpr::List(list, span)))
}

pub fn parse_bool_expr(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();
    let (input, bool) = alt((
        map(parse_word("true"), |_| true),
        map(parse_word("false"), |_| false)
    ))(input)?;
    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, LocatedExpr::Bool(bool, span)))
}

/// Get a factor
pub fn factor(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, neg) = opt(delimited(
        space0,
        alt((tag("!"), parse_word("NOT"))),
        space0
    ))(input)?;

    let (input, not) = opt(delimited(space0, tag("~"), space0))(input)?;

    let (input, factor) = context(
        "[DBG]factor",
        delimited(
            space0,
            context(
                "[DBG]alt",
                alt((
                    prefixed_label_expr,
                    parse_expr_bracketed_list,
                    // Manage functions
                    map(parse_word("RND()"), |w| LocatedExpr::Rnd(w)),
                    parse_unary_function_call,
                    parse_binary_function_call,
                    parse_duration,
                    parse_assemble,
                    parse_any_function_call,
                    // manage values
                    alt((positive_number, negative_number)),
                    char_expr,
                    map(parse_string, |s| LocatedExpr::String(s)),
                    parse_counter,
                    // manage $
                    map(tag("$$"), |l| LocatedExpr::Label(l)),
                    map(tag("$"), |l| LocatedExpr::Label(l)),
                    parse_bool_expr,
                    // manage labels
                    map(parse_label(false), |l| LocatedExpr::Label(l)),
                    parens
                ))
            ),
            space0
        )
    )(input)?;

    let factor = match neg {
        Some(_) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::Neg,
                Box::new(factor),
                input_start.take(input_start.input_len() - input.input_len())
            )
        }
        None => factor
    };

    let factor = match not {
        Some(_) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::Not,
                Box::new(factor),
                input_start.take(input_start.input_len() - input.input_len())
            )
        }
        None => factor
    };

    Ok((input, factor))
}

pub fn negative_number(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, v) = map(preceded(tag("-"), positive_number), |exp| {
        match exp {
            LocatedExpr::Value(v, _) => -v,
            _ => unreachable!()
        }
    })(input)?;

    let span = input_start.take(input_start.len() - input.len());
    Ok((input, LocatedExpr::Value(v, span)))
}

pub fn positive_number(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, number) = terminated(
        alt((hex_number_inner, bin_number_inner, dec_number_inner)),
        not(one_of(
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#@_"
        ))
    )(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, LocatedExpr::Value(number as _, span)))
}

pub fn parse_labelprefix(input: Z80Span) -> IResult<Z80Span, LabelPrefix> {
    alt((
        value(LabelPrefix::Pageset, tag_no_case("{pageset}")),
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}"))
    ))(input)
}

fn fold_exprs(
    initial: LocatedExpr,
    remainder: Vec<(BinaryOperation, LocatedExpr)>,
    span: Z80Span
) -> LocatedExpr {
    remainder.into_iter().fold(initial, move |acc, pair| {
        let (oper, expr) = pair;
        LocatedExpr::BinaryOperation(oper, Box::new(acc), Box::new(expr), span.clone())
    })
}

/// Compute operations related to * % /
pub fn term(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, initial) = factor(input)?;
    let (input, remainder) = my_many0(alt((
        parse_oper(factor, "*", BinaryOperation::Mul),
        parse_oper(factor, "%", BinaryOperation::Mod),
        parse_oper(factor, "/", BinaryOperation::Div)
    )))(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, fold_exprs(initial, remainder, span)))
}

/// Generate a parser of comparison symbol
/// inner: the function the parse the right operand of the symbol
/// pattern: the pattern to match in the source code
/// symbol: the symbol corresponding to the operation
#[inline]
fn parse_oper<F>(
    inner: F,
    pattern: &'static str,
    symbol: BinaryOperation
) -> impl Fn(Z80Span) -> IResult<Z80Span, (BinaryOperation, LocatedExpr), Z80ParserError>
where
    F: Fn(Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError>
{
    #[inline]
    move |input: Z80Span| {
        let (input, _) = space0(input)?;
        let (input, _) = tag_no_case(pattern)(input)?;
        let (input, _) = space0(input)?;
        let (input, operation) = inner(input)?;

        Ok((input, (symbol, operation)))
    }
}
#[inline]
fn parse_bool<F>(
    inner: F,
    pattern: &'static str,
    symbol: BinaryOperation
) -> impl Fn(Z80Span) -> IResult<Z80Span, (BinaryOperation, LocatedExpr), Z80ParserError>
where
    F: Fn(Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError>
{
    #[inline]
    move |input: Z80Span| {
        let (input, _) = space0(input)?;
        let (input, _) = tag_no_case(pattern)(input)?;
        let (input, _) = space0(input)?;
        let (input, operation) = inner(input)?;

        Ok((input, (symbol, operation)))
    }
}

/// Parse an expression
pub fn expr2(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, initial) = shift(input)?;
    let (input, remainder) = my_many0(alt((
        parse_oper(shift, "<=", BinaryOperation::LowerOrEqual),
        parse_oper(shift, "<", BinaryOperation::StrictlyLower),
        parse_oper(shift, ">=", BinaryOperation::GreaterOrEqual),
        parse_oper(shift, ">", BinaryOperation::StrictlyGreater),
        parse_oper(shift, "==", BinaryOperation::Equal),
        parse_oper(shift, "!=", BinaryOperation::Different)
    )))(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, fold_exprs(initial, remainder, span)))
}

fn expr(input: Z80Span) -> IResult<Z80Span, Expr, Z80ParserError> {
    map(located_expr, |e| e.to_expr())(input)
}

/// TODO replace ALL expr parse by a located version
pub fn located_expr(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, initial) = expr2(input)?;
    let (input, remainder) = my_many0(alt((
        parse_oper(expr2, "&&", BinaryOperation::BooleanAnd),
        parse_oper(expr2, "||", BinaryOperation::BooleanOr)
    )))(input)?;
    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, fold_exprs(initial, remainder, span)))
}

/// parse functions with one argument
pub fn parse_unary_function_call(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, (word, exp)) = pair(
        delimited(space0, alpha1, space0),
        context(
            "UNARY function: error in parameters",
            delimited(
                tuple((space0, tag("("), space0)),
                located_expr,
                tuple((space0, tag(")")))
            )
        )
    )(input)?;

    let mut upper_word: smartstring::SmartString<smartstring::Compact> =
        smartstring::SmartString::from(word.as_str());
    upper_word.as_mut_str().make_ascii_uppercase();

    let func = match upper_word.as_str() {
        "HIGH" | "HI" => Some(UnaryFunction::High),
        "LOW" | "LO" => Some(UnaryFunction::Low),
        "PEEK" | "MEMORY" => Some(UnaryFunction::Memory),
        "FLOOR" => Some(UnaryFunction::Floor),
        "CEIL" => Some(UnaryFunction::Ceil),
        "FRAC" => Some(UnaryFunction::Frac),
        "CHAR" => Some(UnaryFunction::Char),
        "INT" => Some(UnaryFunction::Int),
        "SIN" => Some(UnaryFunction::Sin),
        "COS" => Some(UnaryFunction::Cos),
        "ASIN" => Some(UnaryFunction::ASin),
        "ACOS" => Some(UnaryFunction::ACos),
        "LN" => Some(UnaryFunction::Ln),
        "LOG10" => Some(UnaryFunction::Log10),
        "EXP" => Some(UnaryFunction::Exp),
        "SQRT" => Some(UnaryFunction::Char),
        "ABS" => Some(UnaryFunction::Sqrt),
        _ => None
    };

    let span = input_start.take(input_start.input_len() - input.input_len());

    let token = match func {
        Some(func) => LocatedExpr::UnaryFunction(func, Box::new(exp), span),
        None => LocatedExpr::AnyFunction(word, vec![exp], span)
    };

    Ok((input, token))
}

/// parse functions with two arguments
pub fn parse_binary_function_call(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, func) = alt((
        value(BinaryFunction::Min, tag_no_case("MIN")),
        value(BinaryFunction::Max, tag_no_case("MAX")),
        value(BinaryFunction::Pow, tag_no_case("POW"))
    ))(input)?;

    let (input, _) = tuple((space0, tag("("), space0))(input)?;

    let (input, arg1) = located_expr(input)?;
    let (input, _) = tuple((space0, tag(","), space0))(input)?;
    let (input, arg2) = located_expr(input)?;

    let (input, _) = tuple((space0, tag(")")))(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((
        input,
        LocatedExpr::BinaryFunction(func, Box::new(arg1), Box::new(arg2), span)
    ))
}

pub fn parse_any_function_call(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, function_name) = parse_label(false)(input)?;
    let (input, arguments) = delimited(
        tuple((/* space0, */ tag("("), my_space0)),
        separated_list0(parse_comma, located_expr),
        tuple((my_space0, tag(")")))
    )(input)?;


    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((
        input,
        LocatedExpr::AnyFunction(function_name, arguments, span)
    ))
}

/// Parser for functions taking into argument a token
#[inline]
pub fn token_function<'a>(
    function_name: &'static str
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, Z80ParserError> {
    #[inline]
    move |input: Z80Span| {
        let (input, _) = tuple((tag_no_case(function_name), space0, char('('), space0))(input)?;

        let (input, token) = parse_token(input)?;

        let (input, _) = tuple((space0, tag(")")))(input)?;

        Ok((input, token))
    }
}

/// Parse the duration function
pub fn parse_duration(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, token) = token_function("duration")(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((
        input,
        LocatedExpr::UnaryTokenOperation(UnaryTokenOperation::Duration, Box::new(token), span)
    ))
}

/// Parse the single opcode assembling function
pub fn parse_assemble(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, token) = token_function("opcode")(input)?;

    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((
        input,
        LocatedExpr::UnaryTokenOperation(UnaryTokenOperation::Opcode, Box::new(token), span)
    ))
}

pub fn shift(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, initial) = comp(input)?;
    let (input, remainder) = my_many0(alt((
        parse_oper(comp, "<<", BinaryOperation::LeftShift),
        parse_oper(comp, ">>", BinaryOperation::RightShift)
    )))(input)?;
    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, fold_exprs(initial, remainder, span)))
}

/// Parse operation related to + - & |
pub fn comp(input: Z80Span) -> IResult<Z80Span, LocatedExpr, Z80ParserError> {
    let input_start = input.clone();

    let (input, initial) = term(input)?;
    let (input, remainder) = my_many0(alt((
        parse_oper(term, "+", BinaryOperation::Add),
        parse_oper(term, "-", BinaryOperation::Sub),
        parse_oper(term, "&", BinaryOperation::BinaryAnd), /* TODO check if it works and not compete with && */
        parse_oper(term, "AND", BinaryOperation::BinaryAnd),
        parse_oper(term, "|", BinaryOperation::BinaryAnd), /* TODO check if it works and not compete with || */
        parse_oper(term, "OR", BinaryOperation::BinaryOr),
        parse_oper(term, "^", BinaryOperation::BinaryXor), /* TODO check if it works and not compete with ^^ */
        parse_oper(term, "XOR", BinaryOperation::BinaryXor)
    )))(input)?;
    let span = input_start.take(input_start.input_len() - input.input_len());
    Ok((input, fold_exprs(initial, remainder, span)))
}

/// Generate a string from a parsing error. Probably deprecated
#[allow(clippy::needless_pass_by_value)]
pub fn decode_parsing_error(_orig: &str, _e: cpclib_common::nom::Err<&str>) -> String {
    unimplemented!("pub fn decode_parsing_error(orig: &str, e: ::nom::Err<&str>) -> String")
    // let error_string;
    //
    // if let ::nom::Err::Failure(::nom::simple_errors::Context::Code(
    // remaining,
    // ErrorKind::Custom(_),
    // )) = e
    // {
    // let bytes = orig.as_bytes();
    // let complete_size = orig.len();
    // let remaining_size = remaining.input_len();
    // let error_position = complete_size - remaining_size;
    // let line_end = {
    // let mut idx = error_position;
    // while idx < complete_size && bytes[idx] != b'\n' {
    // idx += 1;
    // }
    // idx
    // };
    // let line_start = {
    // let mut idx = error_position;
    // while idx > 0 && bytes[idx - 1] != b'\n' {
    // idx -= 1;
    // }
    // idx
    // };
    //
    // let line = &orig[line_start..line_end];
    // let line_idx = orig[..(error_position)]
    // .bytes()
    // .filter(|b| *b == b'\n')
    // .count(); // way too slow I guess
    // let column_idx = error_position - line_start;
    // let error_description = "Error because";
    // let empty = iter::repeat(" ").take(column_idx).collect::<String>();
    // error_string = format!(
    // "{}:{}:{} {}\n{}\n{}^",
    // "fname", line_idx, column_idx, error_description, line, empty
    // );
    // } else {
    // error_string = String::from("Unknown error");
    // }
    //
    // error_string
}

// Test are deactivated, API is not enough stabilized and tests are broken
#[cfg(test_deactivated)]
mod test {
    use super::*;

    lazy_static::lazy_static! {
        static ref  CTX: ParserContext = ParserContextBuilder::default().build();
    }

    // TODO: remove all its use
    fn ctx() -> &'static ParserContext {
        &CTX
    }

    fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
        let mut ctx = Box::new(ParserContextBuilder::default().build());
        ctx.source = Some(code);
        ctx.context_name = Some("TEST".into());
        let span = Z80Span::new_extra(code, ctx.deref());
        (ctx, span)
    }

    #[test]
    fn test_parse_end_directive() {
        let res = dbg!(parse_end_directive(Z80Span::new_extra("endif", ctx())));
        assert!(res.is_ok());
    }
    #[test]
    fn parse_test_cond() {
        let res = dbg!(inner_code(Z80Span::new_extra(
            " nop
                endif",
            ctx(),
        )));
        assert!(res.is_ok());
        assert_eq!(res.unwrap().1.len(), 1);

        let res = inner_code(Z80Span::new_extra(
            " nop
                else",
            ctx()
        ));
        assert!(res.is_ok());
        assert_eq!(res.unwrap().1.len(), 1);

        let res =
            parse_conditional_condition(KindOfConditional::If)(Z80Span::new_extra("THING", ctx()));
        assert!(res.is_ok());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "if THING
                    nop
                    endif 
                    ",
            ctx()
        ),));
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0.trim());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "if THING
                    nop
                    endif ",
            ctx()
        ),));
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0.trim());

        let res = parse_conditional(Z80Span::new_extra(
            "if THING
                    nop
                    else
                    nop
                    endif",
            ctx()
        ));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = parse_conditional(Z80Span::new_extra(
            "ifndef THING
                    nop
                    else
                    nop
                    endif",
            ctx()
        ));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "if demo_system_music_activated != 0
                    ; XXX Ensure memory is properly set
                    ld bc, 0x7fc2 : out (c), c
                    jp PLY_AKYst_Play
                    else
                    WAIT_CYCLES 64*16
                    ret
                    endif",
            ctx()
        )));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "ifndef __DEFINED_DEBUG__
                    __DEFINED_DEBUG__ equ 1
                    endif",
            ctx()
        )));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = std::dbg!(parse_z80_line(Z80Span::new_extra(
            " ifndef __DEFINED_DEBUG__
                    __DEFINED_DEBUG__ equ 1
                    endif",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", res);
        assert_eq!(b"", res.unwrap().0.as_bytes());
    }

    #[test]
    fn parse_indexregister8() {
        assert_eq!(
            parse_register_ixl(Z80Span::new_extra("ixl", ctx()))
                .unwrap()
                .1,
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert_eq!(
            parse_register_ixl(Z80Span::new_extra("lx", ctx()))
                .unwrap()
                .1,
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert!(parse_register_iyl(Z80Span::new_extra("ixl", ctx())).is_err());
    }

    #[test]
    fn test_parse_prefix_label() {
        let (span, res) = parse_labelprefix(Z80Span::new_extra("{bank}", ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(res, LabelPrefix::Bank);

        let (span, res) = dbg!(expr(Z80Span::new_extra("{bank}label", ctx())).unwrap());
        assert!(span.is_empty());
        assert_eq!(res, Expr::PrefixedLabel(LabelPrefix::Bank, "label".into()));
    }

    #[test]
    fn test_parse_expr_format() {
        let res = formatted_expr(Z80Span::new_extra("{hex} VAL", ctx()));
        assert!(res.is_ok());
        let (span, res) = res.unwrap();
        assert!(span.is_empty());

        assert_eq!(
            res,
            FormattedExpr::Formatted(ExprFormat::Hex(None), Expr::Label("VAL".into()))
        );
    }

    #[test]
    fn test_undocumented_code() {
        let listing = parse_z80_str(" RLC (IY+2), B").unwrap();
        let token = &listing[0];
        let token = token.token().unwrap();
        assert_eq!(
            *token,
            Token::OpCode(
                Mnemonic::Rlc,
                Some(DataAccess::IndexRegister16WithIndex(
                    IndexRegister16::Iy,
                    2.into()
                )),
                Some(DataAccess::Register8(Register8::B)),
                None
            )
        );

        let listing = parse_z80_str(" RES 5, (IY+2), B").unwrap();
        let token = &listing[0];
        let token = token.token().unwrap();
        assert_eq!(
            *token,
            Token::OpCode(
                Mnemonic::Res,
                Some(DataAccess::Expression(5.into())),
                Some(DataAccess::IndexRegister16WithIndex(
                    IndexRegister16::Iy,
                    2.into()
                )),
                Some(Register8::B)
            )
        );
    }

    #[test]
    fn test_parse_print() {
        let (span, res) = parse_print(Z80Span::new_extra("PRINT VAR", ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Raw(Expr::Label("VAR".into()))])
        );

        let (span, res) = parse_print(Z80Span::new_extra("PRINT VAR, VAR", ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![
                FormattedExpr::Raw(Expr::Label("VAR".into())),
                FormattedExpr::Raw(Expr::Label("VAR".into()))
            ])
        );

        let (span, res) = parse_print(Z80Span::new_extra("PRINT {hex}VAR", ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Formatted(
                ExprFormat::Hex(None),
                Expr::Label("VAR".into())
            )])
        );

        let (span, res) = parse_print(Z80Span::new_extra("PRINT \"hello\"", ctx())).unwrap();
        assert!(span.is_empty());

        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Raw(Expr::String("hello".into()))])
        );
    }

    #[test]
    fn test_standard_repeat() {
        let z80 = std::dbg!(
            "  repeat 5
                        nop
                        endrepeat
                        "
        );
        let res = parse_repeat(Z80Span::new_extra(z80, ctx()));
        assert!(res.is_ok(), "{:?}", res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn parser_regression_1() {
        let res = std::dbg!(parse_ld_normal(Z80Span::new_extra(
            "ld a, chessboard_file",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", res);
    }
    #[test]
    fn parser_regression_1a() {
        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let code = unsafe { &*(code.as_str() as *const str) as &'static str };
        let (_ctx, span) = ctx_and_span(code);
        let res = std::dbg!(parse_z80_line_complete(span));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.unwrap().0.as_str(), "                    ");
    }
    #[test]
    fn parser_regression_1b() {
        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let code = unsafe { &*(code.as_str() as *const str) as &'static str };
        let (_ctx, span) = ctx_and_span(code);
        let res = std::dbg!(parse_z80_line(span));
        assert!(res.is_ok(), "{:?}", &res);
        assert!(res.unwrap().0.trim().is_empty());
    }
    #[test]
    fn parser_regression_1c() {
        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let code = unsafe { &*(code.as_str() as *const str) as &'static str };
        let (_ctx, span) = ctx_and_span(code);
        let res = std::dbg!(parse_z80_str(span));
        assert!(res.is_ok(), "{:?}", &res);
        let res = res.unwrap();
        assert_eq!(res.0.len(), 0, "{:?}", &res);
    }
    #[test]
    fn parser_regression_1d() {
        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = inner_code(Z80Span::new_extra(&code.to_owned(), ctx()));
        assert!(res.is_ok(), "{}", &res.err().unwrap().to_string());
        assert_eq!(res.unwrap().0.trim().len(), 0);
    }
    #[test]
    fn parser_regression_1e() {
        let res = std::dbg!(parse_z80_str(
            "
                        ld a, chessboard_file
                        jp .common_part_loading_in_main_memory
                        "
        ));

        assert!(res.is_ok(), "{:?}", &res);
        //   assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }
    #[test]
    fn parser_regression_1f() {
        let res = std::dbg!(inner_code(Z80Span::new_extra(
            "
                        .load_chessboard
                        ld de, .load_chessboard2
                        ld a, main_memory_chessboard_extra_file
                        jp .common_part_loading_in_main_memory
                        .load_chessboard2
                        ld de, .load_chessboard2
                        ld a, main_memory_chessboard_extra_file
                        ld a, chessboard_file
                        jp .common_part_loading_in_main_memory
                        ",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.unwrap().0.trim().len(), 0);
    }
    #[test]
    fn parser_regression_1g() {
        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "if 0
                        .load_chessboard
                        ld de, .load_chessboard2
                        ld a, main_memory_chessboard_extra_file
                        jp .common_part_loading_in_main_memory
                        .load_chessboard2
                        ld de, .load_chessboard2
                        ld a, chessboard_file
                        jp .common_part_loading_in_main_memory
                        
                        endif",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn parser_regression2() {
        let res = std::dbg!(parse_assert(Z80Span::new_extra("assert (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)", ctx())));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn parser_sna() {
        let res = std::dbg!(parse_buildsna(Z80Span::new_extra("BUILDSNA", ctx())));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra("BUILDSNA V2", ctx())));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra("BUILDSNA V3", ctx())));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra("BUILDSNA V4", ctx())));
        assert!(res.is_err(), "{:?}", &res);
    }

    #[test]
    fn test_parse_snaset() {
        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET Z80_SP, 0x500",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET GA_PAL, 0, 30",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET CRTC_REG, 1, 48",
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn test_parse_r16_to_r8() {
        let res = parse_z80_line(Z80Span::new_extra(" ld a, hl.low", ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", res.0);

        let res = parse_ld_normal(Z80Span::new_extra("ld bc.low, a", ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", &res);

        let (span, res) = res;
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::new_opcode(
                Mnemonic::Ld,
                Some(Register8::C.into()),
                Some(Register8::A.into()),
            )
        );

        let res = parse_z80_line(Z80Span::new_extra(" ld bc.low, a", ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", &res);

        let (span, res) = res;
        assert!(span.is_empty());
        assert_eq!(
            res.iter()
                .map(|t| t.token().unwrap())
                .cloned()
                .collect_vec(),
            vec![Token::new_opcode(
                Mnemonic::Ld,
                Some(Register8::C.into()),
                Some(Register8::A.into()),
            )]
        );

        let res = parse_z80_line(Z80Span::new_extra("\t\tld  bc.low, a\n\t", ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", res);
    }
}
