#![allow(clippy::cast_lossless)]

use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;

use choice_nocase::choice_nocase;
use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use cpclib_common::smallvec::SmallVec;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::ascii::{alpha1, alphanumeric1, escaped, line_ending, space0, space1};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, eof, not, opt, peek, preceded, repeat, repeat_till0, separated,
    separated0, separated1, separated_foldl1, terminated
};
use cpclib_common::winnow::error::{
    AddContext, ErrMode, ErrorKind, ParserError, StrContext, VerboseError, VerboseErrorKind
};
use cpclib_common::winnow::stream::{
    Accumulate, AsBStr, AsBytes, AsChar, Offset, Range, Stream, UpdateSlice, Checkpoint
};
use cpclib_common::winnow::token::{
    none_of, one_of, tag, tag_no_case, take, take_till0, take_till1, take_until0, take_while
};
use cpclib_common::winnow::{trace, BStr, PResult, Parser};
use cpclib_common::{lazy_static, winnow};
use cpclib_sna::parse::parse_flag;
use cpclib_sna::{FlagValue, SnapshotVersion};
use cpclib_tokens::ListingElement;
use crc::*;
use either::Either;
use obtained::LocatedTokenInner;

use super::context::*;
use super::obtained::*;
use super::*;
use crate::preamble::*;

const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Z80ParserErrorKind {
    /// Static string added by the `context` function
    Context(StrContext),
    /// Indicates which character was expected by the `char` function
    Char(char),
    /// Error kind given by various nom parsers
    Winnow(ErrorKind),
    /// Chain of errors provided by an inner listing
    Inner {
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Z80ParserError(Vec<(InnerZ80Span, Z80ParserErrorKind)>);

impl Z80ParserError {
    pub fn errors(&self) -> Vec<(&InnerZ80Span, &Z80ParserErrorKind)> {
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
        Self::Winnow(other)
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
            VerboseErrorKind::Context(ctx) => Self::Context(StrContext::Label(ctx)),
            VerboseErrorKind::Winnow(n) => n.into()
        }
    }
}

impl From<VerboseError<InnerZ80Span>> for Z80ParserError {
    fn from(other: VerboseError<InnerZ80Span>) -> Self {
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
        input: &InnerZ80Span,
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    ) -> Self {
        Self(vec![(
            input.clone(),
            Z80ParserErrorKind::Inner { listing, error }
        )])
    }
}

impl ParserError<InnerZ80Span> for Z80ParserError {
    fn from_error_kind(input: &InnerZ80Span, kind: ErrorKind) -> Self {
        Self(vec![(input.clone(), kind.into())])
    }

    fn append(mut self, input: &InnerZ80Span, kind: ErrorKind) -> Self {
        self.0.push((input.clone(), kind.into()));
        self
    }

    fn assert(input: &InnerZ80Span, _message: &'static str) -> Self {
        #[cfg(debug_assertions)]
        panic!("assert `{}` failed at {:#?}", _message, input);
        #[cfg(not(debug_assertions))]
        Self::from_error_kind(input, ErrorKind::Assert)
    }

    fn or(self, other: Self) -> Self {
        other
    }
}

impl AddContext<InnerZ80Span> for Z80ParserError {
    fn add_context(mut self, input: &InnerZ80Span, ctx: &'static str) -> Self {
        self.0.push((
            input.clone(),
            Z80ParserErrorKind::Context(StrContext::Label(ctx))
        ));
        self
    }
}

impl AddContext<InnerZ80Span, &StrContext> for Z80ParserError {
    fn add_context(mut self, input: &InnerZ80Span, ctx: &StrContext) -> Self {
        self.0
            .push((input.clone(), Z80ParserErrorKind::Context(ctx.clone())));
        self
    }
}

impl AddContext<InnerZ80Span, StrContext> for Z80ParserError {
    fn add_context(mut self, input: &InnerZ80Span, ctx: StrContext) -> Self {
        self.0
            .push((input.clone(), Z80ParserErrorKind::Context(ctx)));
        self
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

trait AccumulateSeveral<O>: Accumulate<O> {
    fn accumulate_several(&mut self, items: &mut Vec<O>);
}

impl<O> AccumulateSeveral<O> for Vec<O> {
    fn accumulate_several(&mut self, items: &mut Vec<O>) {
        self.append(items);
    }
}

// TODO search why they are listed to forbid label naming. Delete it if unneeded
const REGISTERS: &[&[u8]] = &[b"AF", b"HL", b"DE", b"BC", b"IX", b"IY", b"IXL", b"IXH"];

const INSTRUCTIONS: &[&[u8]] = &[
    b"ADC", b"ADD", b"AND", b"BIT", b"CALL", b"CCF", b"CP", b"CPD", b"CPDR", b"CPI", b"CPIR",
    b"CPL", b"DAA", b"DEC", b"DI", b"DJNZ", b"EI", b"EX", b"EXX", b"HALT", b"IM", b"IN", b"INC",
    b"IND", b"INDR", b"INI", b"INIR", b"JP", b"JR", b"LD", b"LDD", b"LDDR", b"LDI", b"LDIR",
    b"NEG", b"NOP", b"OR", b"OTDR", b"OTIR", b"OUT", b"OUTD", b"OUTI", b"POP", b"PUSH", b"RES",
    b"RET", b"RETI", b"RETN", b"RL", b"RLA", b"RLC", b"RLCA", b"RLD", b"RR", b"RRA", b"RRC",
    b"RRCA", b"RRD", b"RST", b"SBC", b"SCF", b"SET", b"SLA", b"SRA", b"SRL", b"SUB", b"XOR",
    b"SL1", b"SLL", b"EXA", b"EXD"
];

const STAND_ALONE_DIRECTIVE: &[&[u8]] = &[
    b"#",
    b"ALIGN",
    b"ASSERT",
    b"BANK",
    b"BANKSET",
    b"BINCLUDE",
    b"BREAK",
    b"BREAKPOINT",
    b"BUILDSNA",
    b"BYTE",
    b"CASE",
    b"CHARSET",
    b"DB",
    b"DEFAULT",
    b"DEFB",
    b"DEFM",
    b"DEFS",
    b"DEFW",
    b"DEFSECTION",
    b"DM",
    b"DS",
    b"DW",
    b"ELSE",
    b"END",
    b"ENT",
    b"EQU",
    b"EXPORT",
    b"FAIL",
    b"INCBIN",
    b"INCLUDE",
    b"INCLZ4",
    b"INCEXO",
    b"INCL48",
    b"INCL49",
    b"INCAPU",
    b"INCZX0",
    b"LET",
    b"LIMIT",
    b"LIST",
    b"LZEXO",
    b"MAP",
    b"MODULE",
    b"NOEXPORT",
    b"NOLIST",
    b"NOP",
    b"ORG",
    b"PAUSE",
    b"PRINT",
    b"PROTECT",
    b"RANGE",
    b"READ",
    b"REND",
    b"REPEAT",
    b"REP",
    b"REPT",
    b"RORG",
    b"RETURN",
    b"RUN",
    b"SAVE",
    b"SECTION",
    b"SNAINIT",
    b"SNAPINIT",
    b"SNASET",
    b"STR",
    b"TEXT",
    b"TICKER",
    b"UNDEF",
    b"UNTIL",
    b"WAITNOPS",
    b"WORD",
    b"WRITE DIRECT",
    b"WRITE"
];

const START_DIRECTIVE: &[&[u8]] = &[
    b"CONFINED",
    b"FUNCTION",
    b"FOR",
    b"IF",
    b"IFDEF",
    b"IFEXIST",
    b"IFNDEF",
    b"IFUSED",
    b"ITER",
    b"ITERATE",
    b"LZ4",
    b"LZ48",
    b"LZ49",
    b"LZ48",
    b"LZAPU",
    b"LZX0",
    b"LZEXO",
    b"LZ4",
    b"LZX7",
    b"LOCOMOTIVE",
    b"MACRO",
    b"MODULE",
    b"PHASE",
    b"REPEAT",
    b"REPT",
    b"STRUCT",
    b"SWITCH",
    b"WHILE"
];

// This table is supposed to contain the keywords that finish a section
const END_DIRECTIVE: &[&[u8]] = &[
    b"BREAK",
    b"CASE",
    b"CEND",
    b"DEFAULT",
    b"DEPHASE",
    b"ELSE",
    b"ENDC",
    b"ENDCONFINED",
    b"ENDF",
    b"ENDFOR",
    b"ENDFUNCTION",
    b"ENDI",
    b"ENDIF", // if directive
    b"ENDITER",
    b"ENDITERATE",
    b"ENDM",
    b"ENDMACRO",
    b"ENDMODULE",
    b"ENDR",
    b"ENDREP", // repeat directive
    b"ENDREPEAT",
    b"ENDS",
    b"ENDSWITCH",
    b"ENDW",
    b"FEND",
    b"IEND",
    b"LZCLOSE",
    b"REND", // rorg directive
    b"UNTIL",
    b"WEND"
];

// tODO use hash-based structures
lazy_static::lazy_static! {
    static ref _DOTTED_STAND_ALONE_DIRECTIVE: Vec<String> = STAND_ALONE_DIRECTIVE
                                                .iter()
                                                .map(|d| format!(".{}", unsafe{std::str::from_utf8_unchecked(d)}))
                                                .collect_vec();
    static ref _DOTTED_START_DIRECTIVE: Vec<String> = START_DIRECTIVE
                                                .iter()
                                                .map(|d| format!(".{}", {unsafe{std::str::from_utf8_unchecked(d)}}))
                                                .collect_vec();
    static ref _DOTTED_END_DIRECTIVE: Vec<String> = END_DIRECTIVE
                                                .iter()
                                                .map(|d| format!(".{}", unsafe{std::str::from_utf8_unchecked(d)}))
                                                .collect_vec();
    static ref DOTTED_STAND_ALONE_DIRECTIVE: Vec<&'static [u8]> = _DOTTED_STAND_ALONE_DIRECTIVE.iter().map(String::as_str).map(str::as_bytes).collect_vec();
    static ref DOTTED_START_DIRECTIVE: Vec<&'static [u8]> = _DOTTED_START_DIRECTIVE.iter().map(String::as_str).map(str::as_bytes).collect_vec();
    static ref DOTTED_END_DIRECTIVE: Vec<&'static [u8]> = _DOTTED_END_DIRECTIVE.iter().map(String::as_str).map(str::as_bytes).collect_vec();


    static ref DOTTED_IMPOSSIBLE_NAMES: Vec<&'static [u8]> = REGISTERS
        .into_iter()
        .chain(INSTRUCTIONS.into_iter())
        .chain(DOTTED_STAND_ALONE_DIRECTIVE.iter())
        .chain(DOTTED_START_DIRECTIVE.iter())
        .chain(DOTTED_END_DIRECTIVE.iter())
        .cloned()
        .collect();

    static ref IMPOSSIBLE_NAMES: Vec<&'static [u8]> = REGISTERS
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
    let res = LocatedListing::new_complete_source(str, builder)
        .map_err(|l| AssemblerError::LocatedListingError(std::sync::Arc::new(l)));

    res
}

#[inline]
pub(crate) fn build_span(
    start_eof_offset: usize,
    start: <InnerZ80Span as Stream>::Checkpoint,
    mut input: InnerZ80Span
) -> InnerZ80Span {
    let span_len: usize = start_eof_offset - input.eof_offset();
    input.reset(start);
    let bytes: &'static [u8] = unsafe { std::mem::transmute(&input.as_bstr()[..span_len]) }; // The bytes live longer than input
    input.update_slice(bytes)
}

/// TODO better to build parse_z80_with_options from parse_z80_span than the opposite
// pub fn parse_z80_span(span: InnerZ80Span) -> Result<LocatedListing, AssemblerError> {
//    let ctx = span.extra.clone();
//    parse_z80_with_options(span.as_str(), ctx)
//}

#[inline]
pub fn parse_z80<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_str(code)
}

/// Parse a string and return the corresponding listing
#[inline]
pub fn parse_z80_str<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_with_context_builder(code, ParserContextBuilder::default())
}

#[inline]
/// nom many0 does not seem to fit our parser requirements
/// TODO check if winnow is better on this side
pub fn my_many0<O, E, F, C>(mut f: F) -> impl FnMut(&mut InnerZ80Span) -> PResult<C, E>
where
    F: Parser<InnerZ80Span, O, E>,
    E: ParserError<InnerZ80Span>,
    C: Accumulate<O>
{
    #[inline]
    move |i: &mut InnerZ80Span| {
        let mut acc = C::initial(Some(0));
        let start = i.checkpoint();
        let len = i.eof_offset();

        match f.parse_next(i) {
            Err(ErrMode::Backtrack(_)) => {
                i.reset(start);
                return Ok(acc);
            },
            Err(e) => return Err(e.append(i, ErrorKind::Many)),
            Ok(o) => {
                if len == i.eof_offset() {
                    return Ok(acc); // diff is here
                }
                acc = C::initial(Some(4));
                acc.accumulate(o);
            }
        }

        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();

            match f.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    return Ok(acc);
                },
                Err(e) => return Err(e),
                Ok(o) => {
                    if len == i.eof_offset() {
                        return Ok(acc); // diff is here
                    }
                    acc.accumulate(o);
                }
            }
        }
    }
}

#[inline]
fn my_separated0_in<'vec, O, O2, E, F, G, C>(
    mut sep: G,
    mut f: F,
    r#in: &'vec mut C
) -> impl FnMut(&mut InnerZ80Span) -> PResult<(), E> + 'vec
where
    F: Parser<InnerZ80Span, Either<O, Vec<O>>, E> + 'vec,
    G: Parser<InnerZ80Span, O2, E> + 'vec,
    E: ParserError<InnerZ80Span>,
    C: AccumulateSeveral<O>,
    O: Debug,
    E: Debug,
    O2: Debug
{
    #[inline]
    move |i: &mut InnerZ80Span| {
        let start = i.checkpoint();
        let _len = i.eof_offset();

        //  dbg!("my_separated/start", unsafe{std::str::from_utf8_unchecked(i.as_bytes())});

        match f.parse_next(i) {
            Err(ErrMode::Backtrack(_)) => {
                i.reset(start);
                return Ok(());
            },
            Err(e) => return Err(e.append(i, ErrorKind::Many)),
            Ok(o) => {
                match o {
                    Either::Left(o) => {
                        r#in.accumulate(o);
                    },
                    Either::Right(mut os) => {
                        r#in.accumulate_several(&mut os);
                    }
                }
            },
        }

        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();

            // dbg!("my_separated/next", unsafe{std::str::from_utf8_unchecked(i.as_bytes())});

            match sep.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    return Ok(()); // no pb everything is already in the vec result
                },
                Err(e) => return Err(e.append(i, ErrorKind::Many)),
                Ok(_) => {
                    // infinite loop check: the parser must always consume
                    if i.eof_offset() == len {
                        return Err(ErrMode::assert(i, "`repeat` parsers must always consume"));
                    }

                    let start = i.checkpoint();
                    let _len = i.eof_offset();
                    match f.parse_next(i) {
                        Err(ErrMode::Backtrack(_)) => {
                            i.reset(start); // really usefull ? I doubt
                            return Ok(());
                        },
                        Err(e) => return Err(e),
                        Ok(o) => {
                            match o {
                                Either::Left(o) => {
                                    r#in.accumulate(o);
                                },
                                Either::Right(mut os) => {
                                    r#in.accumulate_several(&mut os);
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

#[inline]
pub fn my_many0_nocollect<O, E, F>(mut f: F) -> impl FnMut(&mut InnerZ80Span) -> PResult<(), E>
where
    F: Parser<InnerZ80Span, O, E>,
    E: ParserError<InnerZ80Span>
{
    #[inline]
    move |i: &mut InnerZ80Span| {
        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();

            match f.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    return Ok(());
                },
                Err(e) => return Err(e),
                Ok(_) => {
                    if len == i.eof_offset() {
                        return Ok(()); // diff is here
                    }
                }
            }
        }
    }
}

#[inline]
pub fn my_many_till_nocollect<O, P, E, F, G>(
    mut f: F,
    mut g: G
) -> impl FnMut(&mut InnerZ80Span) -> PResult<((), P), E>
where
    F: Parser<InnerZ80Span, O, E>,
    G: Parser<InnerZ80Span, P, E>,
    E: ParserError<InnerZ80Span>
{
    #[inline]
    move |i: &mut InnerZ80Span| {
        loop {
            let start_i = i.checkpoint();
            let len = i.eof_offset();
            match g.parse_next(i) {
                Ok(o) => return Ok(((), o)),
                Err(ErrMode::Backtrack(e)) => {
                    match f.parse_next(i) {
                        Err(ErrMode::Backtrack(_err)) => {
                            i.reset(start_i);
                            return Err(ErrMode::Backtrack(e.append(i, ErrorKind::Many)));
                        },
                        Err(e) => return Err(e),
                        Ok(_o) => {
                            // infinite loop check: the parser must always consume
                            if i.eof_offset() == len {
                                return Err(ErrMode::Backtrack(E::from_error_kind(
                                    i,
                                    ErrorKind::Many
                                )));
                            }
                        }
                    }
                },
                Err(e) => return Err(e)
            }
        }
    }
}

#[inline]
fn inner_code(input: &mut InnerZ80Span) -> PResult<LocatedListing, Z80ParserError> {
    inner_code_with_state(input.state.state.clone()).parse_next(input)
}

/// Workaround because many0 is not used in the main root function
/// TODO add an argument to handle context change
#[inline]
pub fn inner_code_with_state(
    new_state: ParsingState
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedListing, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| {
        // dbg!("Requested state", &new_state);
        LocatedListing::parse_inner(input, new_state)
            .map(|l| (Arc::<LocatedListing>::try_unwrap(l).unwrap()))
    }
}

/// TODO
pub fn parse_rorg(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let _ = space0.parse_next(input)?;
    let rorg_start = input.checkpoint();
    let _ = alt((tag_no_case("PHASE"), tag_no_case("RORG"))).parse_next(input)?;

    let exp = delimited(space1, located_expr, space0).parse_next(input)?;

    let _ = my_line_ending.parse_next(input)?;

    let inner = inner_code.parse_next(input)?;

    let _ =
        preceded(space0, alt((tag_no_case("DEPHASE"), tag_no_case("REND")))).parse_next(input)?;

    let _rorg_stop = input.checkpoint();
    let token =
        LocatedTokenInner::Rorg(exp, inner).into_located_token_between(rorg_start, input.clone());
    Ok(token)
}

/// TODO - limit the listing possibilities
pub fn parse_function_listing(input: &mut InnerZ80Span) -> PResult<LocatedListing, Z80ParserError> {
    // dbg!("parse_function_listing requests FunctionLimited state");
    inner_code_with_state(ParsingState::FunctionLimited).parse_next(input)
}

pub fn parse_function(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let function_start = input.checkpoint();
    let _ = preceded(my_space0, parse_directive_word(b"FUNCTION")).parse_next(input)?;
    let name = cut_err(parse_label(false).context("FUNCTION: wrong name")).parse_next(input)?; // TODO use a specific function for that

    let cloned = input.clone();
    let arguments: Vec<InnerZ80Span> = cut_err(
        preceded(
            opt(parse_comma), // comma after macro name is not mandatory
            separated::<_, InnerZ80Span, Vec<InnerZ80Span>, _, _, _, _>(
                0..,
                // parse_label(false)
                delimited(
                    my_space0,
                    take_till1(|c| c == b'\n' || c == b'\r' || c == b':' || c == b',' || c == b' ')
                        .map(|s: &[u8]| cloned.update_slice(s)),
                    my_space0
                ),
                parse_comma
            )
        )
        .context("FUNCTION: errors in parameters")
    )
    .parse_next(input)?;
    let arguments = arguments.into_iter().map(|span| span.into()).collect_vec();

    cut_err(preceded(my_space0, my_line_ending).context("FUNCTION: errors after parameters"))
        .parse_next(input)?;

    let listing =
        cut_err(parse_function_listing.context("FUNCTION: invalid content")).parse_next(input)?;

    let _ = my_many0_nocollect(my_line_ending).parse_next(input)?;
    let _ = alt((
        parse_directive_word(b"ENDF"),
        parse_directive_word(b"ENDFUNCTION")
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Function(name.into(), arguments, listing)
        .into_located_token_between(function_start, input.clone()))
}

/// TODO
pub fn parse_macro(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let dir_start = input.checkpoint();
    let _ = preceded(space0, parse_directive_word(b"MACRO")).parse_next(input)?;

    // macro name
    let name = cut_err(parse_label(false).context("MACRO: wrong name")).parse_next(input)?; // TODO use a specific function for that

    parse_macro_inner(dir_start, name).parse_next(input)
}


fn parse_macro_inner(dir_start: Checkpoint<Checkpoint<Checkpoint<&BStr>>>, name: InnerZ80Span) -> impl FnMut(&mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> +'_ {

    move |input: &mut InnerZ80Span | -> PResult<LocatedToken, Z80ParserError>  {
    // macro arguments
    let arguments = preceded(
        opt(parse_comma), // comma after macro name is not mandatory
        separated::<_, _, Vec<&[u8]>, _, _, _, _>(
            0..,
            // parse_label(false)
            delimited(
                space0,
                take_till1(|c| c == b'\n' || c == b'\r' || c == b':' || c == b',' || c == b' '),
                space0
            ),
            parse_comma
        )
    )
    .parse_next(input)?;
    let arguments = arguments
        .into_iter()
        .map(|span| input.clone().update_slice(span))
        .map(|span| span.into())
        .collect_vec();

    let _ = alt((space0.value(()), my_line_ending.value(()))).parse_next(input)?;

    // TODO factorize with the code of parse_basic
    let before_content = input.checkpoint();
    let (_, end) = cut_err(
        repeat_till0::<_, _, (), _, _, _, _>(
            take(1usize),
            alt((
                parse_directive_word(b"ENDM"),
                parse_directive_word(b"ENDMACRO"),
                parse_directive_word(b"MEND")
            ))
        )
        .context("MACRO: impossible to collect macro content")
    )
    .parse_next(input)?;

    let content_length = end.offset_from(&before_content);
    let mut content = input.clone();
    content.reset(before_content);
    let content: &BStr = unsafe { std::mem::transmute(&content.as_bstr()[..content_length]) };
    let content = input.clone().update_slice(content); // TODO find a way to improve that part. I'd like to not make the conversion

    Ok(LocatedTokenInner::Macro {
        name: name.into(),
        params: arguments,
        content: content.into()
    }
    .into_located_token_between(dir_start, input.clone()))
}
}

/// TODO
pub fn parse_while(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let _ = space0(input)?;
    let while_start = input.checkpoint();
    let _ = parse_directive_word(b"WHILE").parse_next(input)?;

    let cond = cut_err(located_expr.context("WHILE: error in condition")).parse_next(input)?;

    // we must have either a new line or :
    let _ = alt((
        delimited(space0, tag(":"), space0),
        preceded(space0, line_ending)
    ))
    .parse_next(input)?;

    let inner = cut_err(inner_code.context("WHILE: issue in the content")).parse_next(input)?;
    let _ = cut_err(
        preceded(
            space0,
            alt((parse_directive_word(b"ENDW"), parse_directive_word(b"WEND")))
        )
        .context("WHILE: not closed")
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::While(cond, inner)
        .into_located_token_between(while_start, input.clone());
    Ok(token)
}

pub fn parse_module(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let module_start = input.checkpoint();
    let _ = parse_directive_word(b"MODULE").parse_next(input)?;

    let name = cut_err(parse_label(false).context("MODULE: error in naming")).parse_next(input)?;

    let inner = cut_err(inner_code.context("MODULE: issue in the content")).parse_next(input)?;
    let _ =
        cut_err(preceded(space0, parse_directive_word(b"ENDMODULE")).context("MODULE: not closed"))
            .parse_next(input)?;

    let token = LocatedTokenInner::Module(name.into(), inner)
        .into_located_token_between(module_start, input.clone());
    Ok(token)
}

/// Parse a sub-listing part that aims at being crunched after being assembled at first pass
pub fn parse_crunched_section(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let crunched_start = input.checkpoint();
    let kind = preceded(
        space0,
        alt((
            parse_directive_word(b"LZEXO").value(CrunchType::LZEXO),
            parse_directive_word(b"LZ4").value(CrunchType::LZ4),
            parse_directive_word(b"LZ48").value(CrunchType::LZ48),
            parse_directive_word(b"LZ49").value(CrunchType::LZ49),
            parse_directive_word(b"LZX7").value(CrunchType::LZX7),
            parse_directive_word(b"LZX0").value(CrunchType::LZX0),
            parse_directive_word(b"LZAPU").value(CrunchType::LZAPU)
        ))
    )
    .parse_next(input)?;

    let inner =
        cut_err(inner_code.context("CRUNCHED SECTION: issue in the content")).parse_next(input)?;

    let _ = cut_err(
        ((space0, parse_directive_word(b"LZCLOSE"), space0))
            .context("CRUNCHED SECTION section: not closed")
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::CrunchedSection(kind, inner)
        .into_located_token_between(crunched_start, input.clone());
    Ok(token)
}

/// Parse the switch directive
pub fn parse_switch(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let _ =
        my_many0_nocollect(alt((space1.value(()), my_line_ending.value(())))).parse_next(input)?;
    let switch_start = input.clone();
    let _ = parse_directive_word(b"SWITCH")(input)?;

    let value = cut_err(preceded(space0, located_expr).context("SWITCH: tested value"))
        .parse_next(input)?;

    let mut cases_listing = Vec::new();
    let mut default_listing = None;

    loop {
        let _ = cut_err(
            my_many0_nocollect(alt((
                space1,
                line_ending,
                tag(":"),
                parse_comment.recognize()
            )))
            .context("SWITCH: whitespace error")
        )
        .parse_next(input)?;

        // after default it is mandatory to end the block
        let endswitch = if default_listing.is_some() {
            cut_err(
                preceded(
                    space0,
                    alt((
                        parse_directive_word(b"ENDS"),
                        parse_directive_word(b"ENDSWITCH")
                    ))
                    .value(true)
                )
                .context("SWITCH: endswitch not present after default listing.")
            )
            .parse_next(input)?
        }
        else {
            preceded(
                space0,
                opt(alt((
                    parse_directive_word(b"ENDS"),
                    parse_directive_word(b"ENDSWITCH")
                )))
                .map(|e| e.is_some())
            )
            .parse_next(input)?
        };
        if endswitch {
            let token = LocatedTokenInner::Switch(value, cases_listing, default_listing)
                .into_located_token_between(switch_start.checkpoint(), input.clone());
            return Ok(token);
        }

        let value = preceded(my_space0, opt(parse_directive_word(b"CASE"))).parse_next(input)?;
        if value.is_some() {
            let value = cut_err(
                delimited(space0, located_expr, opt(tag(":"))).context("SWITCH: case value error.")
            )
            .parse_next(input)?;

            let inner =
                cut_err(inner_code.context("SWITCH: error in case code")).parse_next(input)?;

            let do_break =
                opt(preceded(space0, parse_directive_word(b"BREAK"))).parse_next(input)?;

            cases_listing.push((value, inner, do_break.is_some()));
        }
        else {
            let _ = cut_err(
                delimited(
                    space0,
                    parse_directive_word(b"DEFAULT"),
                    opt((space0, tag(":")))
                )
                .context("Only CASE, DEFAULT or ENDSWITCH are expected.")
            )
            .parse_next(input)?;
            let default =
                cut_err(inner_code.context("SWITCH: error in default case")).parse_next(input)?;
            default_listing = Some(default);
        }
    }
}

pub fn parse_for(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let for_start = input.checkpoint();
    let _ = preceded(space0, parse_directive_word(b"FOR")).parse_next(input)?;

    // Get parameters
    let counter = cut_err(parse_label(false)).parse_next(input)?;
    let start = cut_err(preceded(parse_comma, located_expr)).parse_next(input)?;
    let stop = cut_err(preceded(parse_comma, located_expr)).parse_next(input)?;
    let step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    // Get loop content
    let inner = cut_err(inner_code.context("FOR: issue in the content")).parse_next(input)?;

    // Collect end of loop
    let _ = cut_err(
        preceded(
            space0,
            alt((
                parse_directive_word(b"ENDFOR"),
                parse_directive_word(b"FEND"),
                parse_directive_word(b"ENDF")
            ))
        )
        .context("FOR: not closed")
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::For {
        label: counter.into(),
        start,
        stop,
        step,
        listing: inner
    }
    .into_located_token_between(for_start, input.clone());
    Ok(token)
}

pub fn parse_confined(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let _ = space0(input)?;
    let confined_start = input.checkpoint();

    let _ = parse_directive_word(b"CONFINED").parse_next(input)?;

    let inner = cut_err(inner_code.context("CONFINED: issue in the content")).parse_next(input)?;

    let _ = cut_err(
        preceded(
            space0,
            alt((
                parse_directive_word(b"ENDCONFINED"),
                parse_directive_word(b"CEND"),
                parse_directive_word(b"ENDC")
            ))
        )
        .context("CONFINED: not closed")
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::Confined(inner)
        .into_located_token_between(confined_start, input.clone());
    Ok(token)
}

pub fn parse_repeat(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let repeat_start = input.checkpoint();
    let _ = preceded(
        space0,
        alt((
            parse_directive_word(b"REP"),
            parse_directive_word(b"REPT"),
            parse_directive_word(b"REPEAT")
        ))
    )
    .parse_next(input)?;

    let count = opt(located_expr).parse_next(input)?;
    match count {
        Some(count) => {
            let counter = cut_err(
                opt(preceded(parse_comma, parse_label(false)))
                    .context("REPEAT: issue in the counter")
            )
            .parse_next(input)?;
            let counter_start = opt(preceded(parse_comma, located_expr)).parse_next(input)?;
            let inner =
                cut_err(inner_code.context("REPEAT: issue in the content")).parse_next(input)?;

            let _ = cut_err(
                preceded(
                    space0,
                    alt((
                        parse_directive_word(b"ENDREPEAT"),
                        parse_directive_word(b"ENDREPT"),
                        parse_directive_word(b"ENDREP"),
                        parse_directive_word(b"ENDR"),
                        parse_directive_word(b"REND")
                    ))
                )
                .context("REPEAT: not closed")
            )
            .parse_next(input)?;

            let token =
                LocatedTokenInner::Repeat(count, inner, counter.map(|c| c.into()), counter_start)
                    .into_located_token_between(repeat_start, input.clone());
            Ok(token)
        },

        None => {
            let inner =
                cut_err(inner_code.context("REPEAT: issue in the content")).parse_next(input)?;

            let _ = cut_err(
                delimited(space0, parse_directive_word(b"UNTIL"), space0)
                    .context("REPEAT ... UNTIL: not closed")
            )
            .parse_next(input)?;
            let cond =
                cut_err(located_expr.context("REPEAT UNTIL: condition error")).parse_next(input)?;
            let token = LocatedTokenInner::RepeatUntil(cond, inner)
                .into_located_token_between(repeat_start, input.clone());
            Ok(token)
        }
    }
}

pub fn parse_iterate(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let iterate_start = input.checkpoint();
    let _ = preceded(
        space0,
        alt((
            parse_directive_word(b"ITERATE"),
            parse_directive_word(b"ITER")
        ))
    )
    .parse_next(input)?;

    let counter =
        cut_err(preceded(space0, parse_label(false)).context("ITERATE: issue in the counter"))
            .parse_next(input)?;

    let comma_or_in = cut_err(
        preceded(my_space0, alt((parse_word(b"IN"), parse_comma)))
            .context("ITERATE: expected ',' or 'in'")
    )
    .parse_next(input)?;

    let values = if comma_or_in.contains(&b',') {
        let values = cut_err(expr_list.context("ITERATE: values issue")).parse_next(input)?;
        either::Either::Left(values)
    }
    else {
        let values = cut_err(
            alt((
                parse_expr_bracketed_list,
                parse_unary_function_call,
                parse_binary_function_call,
                parse_any_function_call,
                parse_assemble,
                parse_label(false).map(|l| LocatedExpr::Label(l.into()))
            ))
            .context("ITERATE: list issue")
        )
        .parse_next(input)?;
        either::Either::Right(values)
    };

    let inner = cut_err(inner_code.context("ITERATE: issue in the content")).parse_next(input)?;

    let _ = cut_err(
        ((
            space0,
            alt((
                parse_directive_word(b"ENDITERATE"),
                parse_directive_word(b"ENDITER"),
                parse_directive_word(b"ENDI"),
                parse_directive_word(b"IEND")
            )),
            space0
        ))
            .context("ITERATE: not closed")
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::Iterate(counter.into(), values, inner)
        .into_located_token_between(iterate_start, input.clone());
    Ok(token)
}

/// TODO
pub fn parse_basic(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let basic_start = input.checkpoint();
    let _ = ((my_space0, tag_no_case("LOCOMOTIVE"), my_space0)).parse_next(input)?;

    // collect the labels that are spread to the basic environnement
    let args: Option<Vec<InnerZ80Span>> = opt(separated(
        1..,
        preceded(my_space0, parse_label(false)),
        parse_comma
    ))
    .parse_next(input)?;
    let args = args.map(|args| {
        args.into_iter()
            .map(|span| Z80Span::from(span))
            .collect_vec()
    });

    (my_space0, opt(line_ending)).parse_next(input)?;

    let hidden_lines = opt(terminated(
        preceded(my_space0, parse_basic_hide_lines),
        my_space0
    ))
    .parse_next(input)?;

    // TODO factorize with the the code of parse_macro
    let before_content = input.checkpoint();
    let (_, end) = cut_err(
        repeat_till0::<_, _, (), _, _, _, _>(take(1usize), parse_directive_word(b"ENDLOCOMOTIVE"))
            .context("BASIC: impossible to collect BASIC content")
    )
    .parse_next(input)?;

    let content_length = end.offset_from(&before_content);
    let mut content = input.clone();
    content.reset(before_content);
    let content: &BStr = unsafe { std::mem::transmute(&content.as_bstr()[..content_length]) };
    let basic = input.clone().update_slice(content); // TODO find a way to improve that part. I'd like to not make the conversion

    let _ = my_space0.parse_next(input)?;

    let token = LocatedTokenInner::Basic(args, hidden_lines, basic.into())
        .into_located_token_between(basic_start, input.clone());
    Ok(token)
}

/// Parse the instruction to hide basic lines
pub fn parse_basic_hide_lines(
    input: &mut InnerZ80Span
) -> PResult<Vec<LocatedExpr>, Z80ParserError> {
    let _ = ((tag_no_case("HIDE_LINES"), space1)).parse_next(input)?;
    expr_list.parse_next(input)
}

pub fn parse_flag_value_inner(input: &mut InnerZ80Span) -> PResult<FlagValue, Z80ParserError> {
    cpclib_sna::parse::parse_flag_value::<InnerZ80Span, Z80ParserError>
        .parse_next(input)
        .map_err(|e| {
            match e {
                ErrMode::Incomplete(_) => todo!(),
                ErrMode::Backtrack(e) => {
                    let mut error = Z80ParserError::from_error_kind(input, ErrorKind::Fail);
                    for ctx in e.context() {
                        error = error.add_context(input, ctx);
                    }

                    ErrMode::Backtrack(error)
                },
                ErrMode::Cut(e) => {
                    let mut error = Z80ParserError::from_error_kind(input, ErrorKind::Fail);
                    for ctx in e.context() {
                        error = error.add_context(input, ctx);
                    }

                    ErrMode::Cut(error)
                }
            }
        })
}


#[inline]
pub fn parse_line_component(
    input: &mut InnerZ80Span
) -> PResult<(Option<LocatedToken>, Option<LocatedToken>), Z80ParserError> {
    my_space0.parse_next(input)?;

    parse_line_component_standard
    .parse_next(input)
}

/// Optionally return a label and a command
/// next  token is a separator :, \n, eof
pub fn parse_line_component_standard(
    input: &mut InnerZ80Span
) -> PResult<(Option<LocatedToken>, Option<LocatedToken>), Z80ParserError> {


    let before_let = input.checkpoint();
    let r#let = terminated(opt(parse_directive_word(b"LET")), my_space0).parse_next(input)?;

    let before_label = input.checkpoint();

    let mut label: Option<InnerZ80Span> = if r#let.is_some() {
        // label is mandatory when there is let
        cut_err(
            parse_label(false)
                .context("LET: missing label")
                .map(|l| Some(l))
        )
        .parse_next(input)?
    }
    else {
        // let was absent
        opt(parse_label(false)).parse_next(input)?
    };

    // build the label token later when needed
    let build_possible_label = move || {
        label.map(|label| LocatedTokenInner::Label(label.into()).into_located_token_direct())
    };

    let before_double_column = input.checkpoint();
    let followed_by_double_column = if label.is_some() {
        opt(':').parse_next(input)?
    }
    else {
        None
    };

    my_space0(input)?;

    // early exit if at the end of the line or if there is a comment
    if r#let.is_none() && input.eof_offset() == 0
        || peek(opt(alt((
            line_ending.value(()),
            ';'.value(()),
            "//".value(())
        ))))
        .parse_next(input)?
        .is_some()
    {
        return Ok((build_possible_label(), None));
    }

    // check if we have a label modifier if and only if we provide a label
    let before_label_modifier = input.checkpoint();
    let label_modifier = if label.is_none() {
        None
    }
    else if r#let.is_some() {
        // LET needs =
        cut_err(b"=".context("LET: missing ="))
            .map(|c| Some(c))
            .parse_next(input)?;
        Some(LabelModifier::Equal(None)) // TODO check it is ok
    }
    else {

        // label can have a modifier
        opt(alt((
                parse_word(b"MACRO").value(LabelModifier::Macro),
                parse_word(b"DEFL").value(LabelModifier::Equ),
                parse_word(b"EQU").value(LabelModifier::Equ),
                parse_word(b"SETN").value(LabelModifier::SetN),
                parse_word(b"NEXT").value(LabelModifier::Next),
                terminated(
                    parse_word(b"SET"),
                    not((my_space0, expr, parse_comma))
                ).map(|_|{
                        LabelModifier::Set
                }),
                b"=".value(LabelModifier::Equal(None)),
                alt((
                    parse_word(b"FIELD").value(()), 
                    b"#".value(())
                )).value(LabelModifier::Field),
                alt((
                        b">>=".value(BinaryOperation::RightShift),
                        b"<<=".value(BinaryOperation::LeftShift),

                        b"+=".value(BinaryOperation::Add),
                        b"-=".value(BinaryOperation::Sub),
                        b"*=".value(BinaryOperation::Mul),
                        b"/=".value(BinaryOperation::Div),
                        b"%=".value(BinaryOperation::Mod),

                        b"&=".value(BinaryOperation::BinaryAnd),
                        b"|=".value(BinaryOperation::BinaryOr),
                        b"^=".value(BinaryOperation::BinaryXor),

                        b"&&=".value(BinaryOperation::BooleanAnd),
                        b"||=".value(BinaryOperation::BooleanOr)
                )).map(|oper|
                        LabelModifier::Equal(Some(oper))
                )
            ))
        ).parse_next(input)?
    };

    if let Some(label_modifier) = label_modifier {
        if label_modifier == LabelModifier::Macro {
            let r#macro = parse_macro_inner(before_label, label.unwrap())
                .context("MACRO: error on macro definition")
        .parse_next(input)?;
            return Ok((None, Some(r#macro)));

        }

        let expr_arg = match &label_modifier {
            LabelModifier::Equ
            | LabelModifier::Equal(..)
            | LabelModifier::Set
            | LabelModifier::Field => {
                cut_err(located_expr.map(|e| Some(e)))
                    .context("Value error")
                    .parse_next(input)?
            },
            _ => None
        };

        let source_label = match &label_modifier {
            LabelModifier::Next | LabelModifier::SetN => {
                cut_err(
                    preceded(my_space0, parse_label(false))
                        .map(|l| Some(l))
                        .context("Label expected")
                )
                .parse_next(input)?
            },
            _ => None
        };

        // optional expression to control the displacement
        let additional_arg = match &label_modifier {
            LabelModifier::Next | LabelModifier::SetN => {
                opt(preceded(parse_comma, located_expr)).parse_next(input)?
            },
            _ => None
        };

        debug_assert!(label.is_some());
        let label = unsafe { label.unwrap_unchecked() };

        // Build the needed token for the label of interest
        let token: LocatedToken = match label_modifier {
            LabelModifier::Equ => {
                LocatedTokenInner::Equ {
                    label: label.into(),
                    expr: expr_arg.unwrap()
                }
            },
            LabelModifier::Equal(op) => {
                LocatedTokenInner::Assign {
                    label: label.into(),
                    expr: expr_arg.unwrap(),
                    op
                }
            },
            LabelModifier::Set => {
                LocatedTokenInner::Assign {
                    label: label.into(),
                    expr: expr_arg.unwrap(),
                    op: None
                }
            },
            LabelModifier::SetN => {
                LocatedTokenInner::SetN {
                    label: label.into(),
                    source: source_label.unwrap().into(),
                    expr: additional_arg
                }
            },
            LabelModifier::Next => {
                LocatedTokenInner::Next {
                    label: label.into(),
                    source: source_label.unwrap().into(),
                    expr: additional_arg
                }
            },
            LabelModifier::Field => {
                LocatedTokenInner::Field {
                    label: label.into(),
                    expr: expr_arg.unwrap().into()
                }
            },
        LabelModifier::Macro => unreachable!("This case must have been handled before")

        }
        .into_located_token_between(before_label, input.clone());

        Ok((None, Some(token)))
    }
    else {
        // ensure we have not eaten some label modifier bytes in case of error
        input.reset(before_label_modifier);

        // if a label was present as well as :, we prefer to stop here
        if label.is_some() && followed_by_double_column.is_some() {
            input.reset(before_double_column);
            return Ok((build_possible_label(), None));
        }

        // otherwise this is a normal stuff

        // we must have an instruction if label is missing; otherwise it is optional
        let instruction =
            opt(alt((parse_z80_directive_with_block, parse_single_token))).parse_next(input)?;

        if label.is_some() && instruction.is_none() {
            if let Ok(call) = parse_macro_or_struct_call_inner(false, label.take().unwrap()) // label is eaten
                .map(|m| Some(m))
                .parse_next(input)
            {
                // this is a macro call
                let call = call.map(|t| t.into_located_token_between(before_label, input.clone()));
                my_space0.parse_next(input)?;

                return Ok((None, call));
            }
            else {
                // this is a label
                return Ok((build_possible_label(), None));
            }
        }
        else {
            // this cannot be a macro as there is an instruction
            my_space0.parse_next(input)?;
            return Ok((build_possible_label(), instruction));
        }
    }
}

/// TODO - currently consume several lines. Should do it only one time
#[inline]
pub fn parse_line_or_with_comment(
    input: &mut InnerZ80Span
) -> PResult<Option<LocatedToken>, Z80ParserError> {
    // let _ =opt(line_ending).parse_next(input)?;
    let _before_comment = input.clone();
    let comment = delimited(space0, opt(parse_comment), space0).parse_next(input)?;
    let _ = alt((line_ending, eof)).parse_next(input)?;

    // let res = if comment.is_some() {
    // let size = before_comment.input_len() - input.input_len();
    // Some(comment.unwrap().locate(before_comment, size))
    // }
    // else {
    // None
    // };
    Ok(comment)
}

#[inline]
fn parse_single_token(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    // Get the token
    alt((parse_token, parse_directive)).parse_next(input)
}

// TODO add struct and Macro
#[derive(Clone, Copy, Debug, PartialEq)]
enum LabelModifier {
    Equ,
    Set,
    Equal(Option<BinaryOperation>),
    SetN,
    Next,
    Field,
    Macro
}

pub fn parse_fname(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
        parse_string(input)
}

#[inline]
pub fn parse_z80_directive_with_block(
    input: &mut InnerZ80Span
) -> PResult<LocatedToken, Z80ParserError> {
    alt((
        parse_basic.context("Basic code embedding"),
        parse_macro,
        parse_crunched_section,
        parse_module,
        parse_confined,
        parse_repeat,
        parse_for,
        parse_function,
        parse_switch,
        parse_iterate,
        parse_while,
        parse_rorg,
        parse_conditional
    ))
    .parse_next(input)
}

#[inline]
pub fn parse_lines(input: &mut InnerZ80Span) -> PResult<Vec<LocatedToken>, Z80ParserError> {
    let mut tokens = Vec::with_capacity(100);

    loop {
        let offset = input.eof_offset();
        let res = opt(parse_z80_line_complete(&mut tokens)).parse_next(input)?;
        if res.is_none() || offset == input.eof_offset() {
            break;
        }
    }

    Ok(tokens)
}

/// Parse a line (ie a set of components separated by :) until the end of the line or a stop directive
/// XXX: In opposite to the other functions, the result is stored in the parameter (to avoid unnecessary memory allocations and copies)
#[inline]
pub fn parse_line(
    r#in: &mut Vec<LocatedToken>
) -> impl FnMut(&mut InnerZ80Span) -> PResult<(), Z80ParserError> + '_ {
    move |input: &mut InnerZ80Span| -> PResult<(), Z80ParserError> {
        my_space0.parse_next(input)?;

        let mut components: SmallVec<[_; 1]> = Default::default();
        loop {
            let local = opt(parse_line_component).parse_next(input)?;
            if let Some(local) = local {
                components.push(local);
            }
            else {
                break; //  macro content ?
            }

            my_space0.value(()).parse_next(input)?;

            let delim = opt((':', my_space0.value(())).value(())).parse_next(input)?;
            if delim.is_none() {
                break;
            }
        }

        // early stop parsing in case of stop directive
        let before_end = input.checkpoint();
        let stop = opt(parse_end_directive).parse_next(input)?;
        let comment = if stop.is_some() {
            input.reset(before_end);
            None
        }
        else {
            let comment = opt(parse_comment).parse_next(input)?;

            alt((eof::<_, Z80ParserError>, line_ending))
                .value(())
                .context("Line ending expected")
                .parse_next(input)?;

            comment
        };

        // Inject the list of instructions
        for (label, instruction) in components.into_iter() {
            if let Some(label) = label {
                r#in.push(label);
            }
            if let Some(instruction) = instruction {
                r#in.push(instruction)
            }
        }

        // Inject the comment
        if let Some(comment) = comment {
            r#in.push(comment);
        }

        Ok(())
    }
}

pub fn parse_z80_line_complete(
    r#in: &mut Vec<LocatedToken>
) -> impl FnMut(&mut InnerZ80Span) -> PResult<(), Z80ParserError> + '_ {
    parse_line(r#in)
}


#[inline]
pub fn parse_assign_operator(
    input: &mut InnerZ80Span
) -> PResult<Option<BinaryOperation>, Z80ParserError> {
    let word = take_while(1..=3, |c| {
        c == b'='
            || c == b'<'
            || c == b'>'
            || c == b'+'
            || c == b'-'
            || c == b'*'
            || c == b'/'
            || c == b'%'
            || c == b'^'
            || c == b'|'
            || c == b'&'
    })
    .parse_next(input)?;
    let oper = match word {
        b"=" => None,

        b">>=" => Some(BinaryOperation::RightShift),
        b"<<=" => Some(BinaryOperation::LeftShift),

        b"+=" => Some(BinaryOperation::Add),
        b"-=" => Some(BinaryOperation::Sub),
        b"*=" => Some(BinaryOperation::Mul),
        b"/=" => Some(BinaryOperation::Div),
        b"%=" => Some(BinaryOperation::Mod),

        b"&=" => Some(BinaryOperation::BinaryAnd),
        b"|=" => Some(BinaryOperation::BinaryOr),
        b"^=" => Some(BinaryOperation::BinaryXor),

        b"&&=" => Some(BinaryOperation::BooleanAnd),
        b"||=" => Some(BinaryOperation::BooleanOr),

        _ => {
            return Err(ErrMode::Cut(
                Z80ParserError::from_error_kind(input, ErrorKind::Alt)
                    .add_context(input, "Wrong symbol")
            ))
        },
    };

    Ok(oper)
}


/// Parser for file names in appropriate directives
#[inline]
pub fn parse_string(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let first = alt(('"', '\'')).parse_next(input)? as char;
    let last = first;
    let (normal, escapable) = match first {
        '\'' => (none_of(('\\', '\'')), one_of(('\\', '\''))),
        '"' => (none_of(('\\', '"')), one_of(('\\', '"'))),
        _ => unreachable!()
    };

    let content = alt((
        last.recognize(),
        terminated(
            escaped(normal, '\\', escapable),
            last.context("End of string not found")
        )
    ))
    .parse_next(input)?;

    let string = if content.len() == 1 && first == (content[0] as char) {
        &content[..0] // we remove " (it is not present for the others)
    }
    else {
        &content[..]
    };

    let string = input.clone().update_slice(string);

    Ok(string)
}

pub fn parse_charset(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let charset =
        opt(alt((parse_charset_string, parse_charset_start_stop_end))).parse_next(input)?;

    Ok(charset
        .map(|c| LocatedTokenInner::Charset(c))
        .unwrap_or_else(|| LocatedTokenInner::Charset(CharsetFormat::Reset)))
}

pub fn parse_charset_start_stop_end(
    input: &mut InnerZ80Span
) -> PResult<CharsetFormat, Z80ParserError> {
    let (start, stop, end) = ((
        expr,
        preceded(parse_comma, expr),
        opt(preceded(parse_comma, expr))
    ))
        .parse_next(input)?;

    let format = if let Some(end) = end {
        CharsetFormat::Interval(start, stop, end)
    }
    else {
        CharsetFormat::Char(start, stop)
    };
    Ok(format)
}

pub fn parse_charset_string(input: &mut InnerZ80Span) -> PResult<CharsetFormat, Z80ParserError> {
    // manage the string format - TODO manage the others too
    let chars = parse_string.context("Missing string").parse_next(input)?;
    let chars = unsafe { std::str::from_utf8_unchecked(&chars) };
    let start = preceded(parse_comma, expr)
        .context("Missing start value")
        .parse_next(input)?;
    let format = CharsetFormat::CharsList(chars.chars().collect_vec(), start);

    Ok(format)
}

/// Parser for the include directive
pub fn parse_include(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let once_fname = (
        opt(delimited(space0, parse_word(b"ONCE"), space0)),
        parse_fname
    )
        .parse_next(input)?;

    let (once, fname) = once_fname;

    let namespace = opt(preceded(
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
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Include(
        fname.into(),
        namespace.map(|n| n.into()),
        once.is_some()
    ))
}

/// Parse for the various binary include directives
#[inline]
pub fn parse_incbin(
    transformation: BinaryTransformation
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let fname = preceded(space0, parse_fname).parse_next(input)?;

        let offset = opt(preceded((space0, (','), space0), located_expr)).parse_next(input)?;
        let length = opt(preceded((space0, (','), space0), located_expr)).parse_next(input)?;
        let _extended_offset = opt(preceded((space0, (','), space0), expr)).parse_next(input)?;
        let off = opt(preceded((space0, (','), space0), tag_no_case("OFF"))).parse_next(input)?;

        Ok(LocatedTokenInner::Incbin {
            fname: fname.into(),
            offset,
            length,
            extended_offset: None,
            off: off.is_some(),
            transformation
        })
    }
}

/// parse write direct in memory / converted to a bank directive
/// we do not care of the parameters for roms as we are not working in an emulator
pub fn parse_write_direct_memory(
    input: &mut InnerZ80Span
) -> PResult<LocatedTokenInner, Z80ParserError> {
    // filter all the stuff before
    let _ = ((
        tag_no_case("DIRECT"),
        space1,
        tag_no_case("-1"),
        parse_comma,
        tag_no_case("-1"),
        parse_comma
    ))
        .parse_next(input)?;

    let bank = located_expr(input)?;

    let token = LocatedTokenInner::Bank(Some(bank));

    Ok(LocatedTokenInner::WarningWrapper(
        Box::new(token),
        "Prefer BANK or PAGE directives to write direct -1, -1, XX".to_owned()
    ))
}

#[derive(PartialEq)]
pub enum SaveKind {
    Save,
    WriteDirect
}

/// Parse both save directive and write direct in a file
pub fn parse_save(
    save_kind: SaveKind
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if save_kind == SaveKind::WriteDirect {
            parse_word(b"DIRECT").parse_next(input)?;
        }

        let filename = parse_fname.parse_next(input)?;

        let address = opt(preceded(parse_comma, opt(located_expr))).parse_next(input)?;

        let size = if address.is_some() {
            opt(preceded(parse_comma, opt(located_expr))).parse_next(input)?
        }
        else {
            None
        };

        let save_type = if size.is_some() && save_kind == SaveKind::Save {
            opt(preceded(
                parse_comma,
                alt((
                    parse_word(b"AMSDOS").value(SaveType::AmsdosBin),
                    parse_word(b"BASIC").value(SaveType::AmsdosBas),
                    parse_word(b"DSK").value(SaveType::Disc(DiscType::Dsk)),
                    parse_word(b"HFE").value(SaveType::Disc(DiscType::Hfe)),
                    parse_word(b"DISC").value(SaveType::Disc(DiscType::Auto)),
                    parse_word(b"TAPE").value(SaveType::Tape)
                ))
            ))
            .parse_next(input)?
        }
        else {
            if save_kind == SaveKind::WriteDirect {
                Some(SaveType::AmsdosBin)
            }
            else {
                None
            }
        };

        let dsk_filename = if save_type.is_some() && save_kind == SaveKind::Save {
            opt(preceded(parse_comma, parse_fname)).parse_next(input)?
        }
        else {
            None
        };

        let side = if dsk_filename.is_some() && save_kind == SaveKind::Save {
            opt(preceded(parse_comma, located_expr)).parse_next(input)?
        }
        else {
            None
        };

        Ok(LocatedTokenInner::Save {
            filename: filename.into(),
            address: address.unwrap_or(None),
            size: size.unwrap_or(None),
            save_type,
            dsk_filename: dsk_filename.map(|f| f.into()),
            side
        })
    }
}

/// Parse  UNDEF directive.
pub fn parse_undef(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let label = parse_label(false).parse_next(input)?;

    Ok(LocatedTokenInner::Undef(label.into()))
}

pub fn parse_section(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let name = preceded(space0, parse_label(false)).parse_next(input)?;

    Ok(LocatedTokenInner::Section(name.into()))
}

#[inline]
pub fn parse_range(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let start =
        cut_err(delimited(space0, located_expr, space0).context("RANGE: wrong start address"))
            .parse_next(input)?;
    let stop = cut_err(
        preceded(parse_comma, delimited(space0, located_expr, space0))
            .context("RANGE: wrong end address")
    )
    .parse_next(input)?;
    let label = cut_err(
        preceded(parse_comma, delimited(space0, parse_label(false), space0))
            .context("RANGE: wrong name")
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Range(label.into(), start, stop))
}
// pub fn parse_assign(input: &mut InnerZ80Span) -> PResult<TokenInner, Z80ParserError> {
// let ((label, op, value)) = ((
// parse_label(false),
// delimited(space0, parse_assign_operator, space0),
// expr
// )).parse_next(input)?;
//
// Ok((TokenInner::Assign{label, value, op}))
// }

#[inline]
pub fn parse_token(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let parsing_state = input.state.state.clone();

    alt((parse_token1, parse_token2))
        .verify(move |t| t.is_accepted(&parsing_state))
        .parse_next(input)
}

#[inline]
pub fn parse_token1(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    parse_opcode_no_arg(input)
}

#[inline]
pub fn parse_token2(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let input_start = input.checkpoint();

    // Get the first word that will drive the rest of parsing
    let word = delimited(space0, alpha1, space0).parse_next(input)?;

    // Apply the right parsing
    // We use this way of doing to reduce function calls and error. Let's hope it will speed everything
    // choice_no_case is used to avoid memory allocation of uppercased mnemonic
    let token: LocatedTokenInner = match word {
        choice_nocase!(b"LD") => parse_ld(true).parse_next(input),
        choice_nocase!(b"ADC") => parse_add_or_adc(Mnemonic::Adc).parse_next(input),
        choice_nocase!(b"ADD") => parse_add_or_adc(Mnemonic::Add).parse_next(input),
        choice_nocase!(b"AND") => parse_logical_operator(Mnemonic::And).parse_next(input),

        choice_nocase!(b"BIT") => parse_res_set_bit(Mnemonic::Bit).parse_next(input),

        choice_nocase!(b"CALL") => parse_call_jp_or_jr(Mnemonic::Call).parse_next(input),
        choice_nocase!(b"CP") => parse_cp.parse_next(input),

        choice_nocase!(b"DEC") => parse_inc_dec(Mnemonic::Dec).parse_next(input),
        choice_nocase!(b"DJNZ") => parse_djnz.parse_next(input),

        choice_nocase!(b"EX") => {
            alt((parse_ex_af, parse_ex_hl_de, parse_ex_mem_sp)).parse_next(input)
        },

        choice_nocase!(b"EXA") => Ok(LocatedTokenInner::new_opcode(Mnemonic::ExAf, None, None)),
        choice_nocase!(b"EXD") => Ok(LocatedTokenInner::new_opcode(Mnemonic::ExHlDe, None, None)),

        choice_nocase!(b"IN") => parse_in.parse_next(input),
        choice_nocase!(b"INC") => parse_inc_dec(Mnemonic::Inc).parse_next(input),
        choice_nocase!(b"IM") => parse_im.parse_next(input),

        choice_nocase!(b"JP") => parse_call_jp_or_jr(Mnemonic::Jp).parse_next(input),
        choice_nocase!(b"JR") => parse_call_jp_or_jr(Mnemonic::Jr).parse_next(input),

        choice_nocase!(b"OR") => parse_logical_operator(Mnemonic::Or).parse_next(input),
        choice_nocase!(b"OUT") => parse_out.parse_next(input),

        choice_nocase!(b"POP") => parse_push_n_pop(Mnemonic::Pop).parse_next(input),
        choice_nocase!(b"PUSH") => parse_push_n_pop(Mnemonic::Push).parse_next(input),

        choice_nocase!(b"RES") => parse_res_set_bit(Mnemonic::Res).parse_next(input),
        choice_nocase!(b"RET") => parse_ret.parse_next(input),
        choice_nocase!(b"RLC") => parse_shifts_and_rotations(Mnemonic::Rlc).parse_next(input),
        choice_nocase!(b"RL") => parse_shifts_and_rotations(Mnemonic::Rl).parse_next(input),
        choice_nocase!(b"RRC") => parse_shifts_and_rotations(Mnemonic::Rrc).parse_next(input),
        choice_nocase!(b"RR") => parse_shifts_and_rotations(Mnemonic::Rr).parse_next(input),
        choice_nocase!(b"RST") => parse_rst.parse_next(input),

        choice_nocase!(b"SBC") => parse_sbc.parse_next(input),
        choice_nocase!(b"SET") => parse_res_set_bit(Mnemonic::Set).parse_next(input),
        choice_nocase!(b"SL1") => parse_shifts_and_rotations(Mnemonic::Sl1).parse_next(input),
        choice_nocase!(b"SLA") => parse_shifts_and_rotations(Mnemonic::Sla).parse_next(input),
        choice_nocase!(b"SLL") => parse_shifts_and_rotations(Mnemonic::Sl1).parse_next(input),
        choice_nocase!(b"SRA") => parse_shifts_and_rotations(Mnemonic::Sra).parse_next(input),
        choice_nocase!(b"SRL") => parse_shifts_and_rotations(Mnemonic::Srl).parse_next(input),
        choice_nocase!(b"SUB") => parse_sub.parse_next(input),

        choice_nocase!(b"XOR") => parse_logical_operator(Mnemonic::Xor).parse_next(input),

        _ => {
            Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Alt
            )))
        },
    }?;

    let token = token.into_located_token_between(input_start, input.clone());
    Ok(token)
}

/// Parse ex af, af' instruction
#[inline]
pub fn parse_ex_af(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    ((
        //        parse_word(b"EX"),
        parse_register_af,
        parse_comma,
        parse_word(b"AF'")
    ))
        .map(|_| LocatedTokenInner::new_opcode(Mnemonic::ExAf, None, None))
        .parse_next(input)
}

/// Parse ex hl, de instruction
#[inline]
pub fn parse_ex_hl_de(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    alt((
        ((
            //          tag_no_case("EX"),
            //          space1,
            parse_register_hl,
            parse_comma,
            parse_register_de
        ))
            .value(()),
        ((
            //            tag_no_case("EX"),
            //        space1,
            parse_register_de,
            parse_comma,
            parse_register_hl
        ))
            .value(())
    ))
    .map(|_| LocatedTokenInner::new_opcode(Mnemonic::ExHlDe, None, None))
    .parse_next(input)
}

/// Parse ex (sp), hl
#[inline]
pub fn parse_ex_mem_sp(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let destination = ((
        //     tag_no_case("EX"),
        //      space1,
        ('('),
        space0,
        parse_register_sp,
        space0,
        (')'),
        parse_comma,
        alt((parse_register_hl, parse_indexregister16))
    ))
        .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::ExMemSp,
        Some(destination.6),
        None
    ))
}

#[inline]
pub fn parse_struct_directive(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    alt((
        parse_struct_directive_inner,
        parse_macro_or_struct_call(false, true)
    ))
    .parse_next(input)
}

#[inline]
fn parse_struct_directive_inner(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    // XXX Sadly the state is stored within the context that cannot
    //     by changed. So we can cannot really use parsing state sutf

    let input_start = input.checkpoint();
    let parsing_state = ParsingState::StructLimited;
    let directive = parse_directive_new(&parsing_state.clone())
        .verify(move |d| d.is_accepted(&parsing_state))
        .parse_next(input)?;

    // Only one argument is allowed
    if (directive.is_db() || directive.is_dw()) && directive.data_exprs().len() > 1 {
        input.reset(input_start);

        return Err(ErrMode::Cut(
            Z80ParserError::from_error_kind(input, ErrorKind::Many)
                .add_context(input, "0 or 1 arguments are expected")
                .into()
        ));
    }
    Ok(directive)
}

/// Parse any directive
pub fn parse_directive(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let parsing_state = input.state.state.clone();
    parse_directive_new(&parsing_state.clone())
        .verify(move |d| d.is_accepted(&parsing_state))
        .parse_next(input)
}

#[inline]
/// Here local_parsing_state only serves to adapt DB/DW/STR behavior in struct.
/// Maybe it should be used to control the directives of interest BEFORE there parsing instead of after.
/// No filtering is done
pub fn parse_directive_new(
    local_parsing_state: &ParsingState
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> + '_ {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedToken, Z80ParserError> {
        let input_start = input.checkpoint();

        // Get the first word that will drive the rest of parsing
        let word = delimited(
            my_space0,
            terminated(
                alphanumeric1,
                alt((eof.value(()), not(alt((b'.', b'_'))).value(())))
            ),
            my_space0
        )
        .parse_next(input)?;

        let within_struct = local_parsing_state == &ParsingState::StructLimited;

        //   dbg!("Directive:", unsafe{std::str::from_utf8_unchecked(word)});

        let token: LocatedTokenInner = match word {
            choice_nocase!(b"ORG") => parse_org.parse_next(input)?,

            choice_nocase!(b"DB")
            | choice_nocase!(b"DEFB")
            | choice_nocase!(b"DM")
            | choice_nocase!(b"DEFM")
            | choice_nocase!(b"BYTE")
            | choice_nocase!(b"TEXT") => {
                parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)?
            },
            choice_nocase!(b"WORD") | choice_nocase!(b"DW") | choice_nocase!(b"DEFW") => {
                parse_db_or_dw_or_str(DbDwStr::Dw, within_struct).parse_next(input)?
            },
            choice_nocase!(b"STR") => {
                parse_db_or_dw_or_str(DbDwStr::Str, within_struct).parse_next(input)?
            },

            choice_nocase!(b"INCBIN") | choice_nocase!(b"BINCLUDE") => {
                parse_incbin(BinaryTransformation::None).parse_next(input)?
            },
            choice_nocase!(b"INCEXO") => {
                parse_incbin(BinaryTransformation::Crunch(CrunchType::LZEXO)).parse_next(input)?
            },
            choice_nocase!(b"INCLZ4") => {
                parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ4)).parse_next(input)?
            },
            choice_nocase!(b"INCL48") => {
                parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ48)).parse_next(input)?
            },
            choice_nocase!(b"INCL49") => {
                parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ49)).parse_next(input)?
            },
            choice_nocase!(b"INCAPU") => {
                parse_incbin(BinaryTransformation::Crunch(CrunchType::LZAPU)).parse_next(input)?
            },
            choice_nocase!(b"INCZX0") => {
                parse_incbin(BinaryTransformation::Crunch(CrunchType::LZX0)).parse_next(input)?
            },

            choice_nocase!(b"INCLUDE") | choice_nocase!(b"READ") => {
                parse_include.parse_next(input)?
            },

            choice_nocase!(b"STRUCT") => parse_struct.parse_next(input)?,
            choice_nocase!(b"SAVE") => parse_save(SaveKind::Save).parse_next(input)?,
            choice_nocase!(b"WRITE") => {
                alt((parse_save(SaveKind::WriteDirect), parse_write_direct_memory))
                    .parse_next(input)?
            },
            choice_nocase!(b"FILL")
            | choice_nocase!(b"DS")
            | choice_nocase!(b"DEFS")
            | choice_nocase!(b"RMEM") => parse_defs.parse_next(input)?,

            choice_nocase!(b"ALIGN") => parse_align.parse_next(input)?,
            choice_nocase!(b"ASSERT") => parse_assert.parse_next(input)?,

            choice_nocase!(b"BANK") => parse_bank.parse_next(input)?,
            choice_nocase!(b"BANKSET") => parse_bankset.parse_next(input)?,
            choice_nocase!(b"BREAKPOINT") => parse_breakpoint.parse_next(input)?,
            choice_nocase!(b"BUILDSNA") => parse_buildsna(true).parse_next(input)?,

            choice_nocase!(b"CHARSET") => parse_charset.parse_next(input)?,

            choice_nocase!(b"DEFSECTION") => parse_range.parse_next(input)?,

            choice_nocase!(b"END") => Ok(LocatedTokenInner::End)?,
            choice_nocase!(b"ENT") => parse_run(RunEnt::Ent).parse_next(input)?,

            choice_nocase!(b"EXPORT") => parse_export(ExportKind::Export).parse_next(input)?,

            choice_nocase!(b"FAIL") => parse_fail(true).parse_next(input)?,

            choice_nocase!(b"LIMIT") => parse_limit.parse_next(input)?,
            choice_nocase!(b"LIST") => Ok(LocatedTokenInner::List)?,

            choice_nocase!(b"MAP") => parse_map.parse_next(input)?,

            choice_nocase!(b"NOEXPORT") => parse_export(ExportKind::NoExport).parse_next(input)?,
            choice_nocase!(b"NOLIST") => LocatedTokenInner::NoList,
            choice_nocase!(b"NOP") => parse_nop.parse_next(input)?,

            choice_nocase!(b"PAUSE") => Ok(LocatedTokenInner::Pause)?,
            choice_nocase!(b"PRINT") => parse_print(true).parse_next(input)?,
            choice_nocase!(b"PROTECT") => parse_protect.parse_next(input)?,

            choice_nocase!(b"RANGE") => parse_range.parse_next(input)?,
            choice_nocase!(b"RETURN") => parse_return.parse_next(input)?,
            choice_nocase!(b"RUN") => parse_run(RunEnt::Run).parse_next(input)?,

            choice_nocase!(b"SECTION") => parse_section.parse_next(input)?,
            choice_nocase!(b"SNASET") => parse_snaset(true).parse_next(input)?,

            choice_nocase!(b"SNAPINIT") | choice_nocase!(b"SNAINIT") => {
                parse_snainit.parse_next(input)?
            },

            choice_nocase!(b"TICKER") => parse_stable_ticker.parse_next(input)?,

            choice_nocase!(b"UNDEF") => parse_undef.parse_next(input)?,

            choice_nocase!(b"WAITNOPS") => parse_waitnops.parse_next(input)?,

            _ => {
                input.reset(input_start);
                return Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
                    input,
                    ErrorKind::Alt
                )));
            }
        };

        let token = token.into_located_token_between(input_start, input.clone());
        Ok(token)
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
#[inline]
pub fn parse_conditional(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let if_start = input.checkpoint();

    let mut conditions = Vec::new();
    let mut else_clause = None;

    loop {
        let first_loop = conditions.is_empty();

        // Gest the kind of test to do - it can fail after an else
        let if_token_or_error = alt((
            parse_directive_word(b"IF").value(KindOfConditional::If),
            parse_directive_word(b"IFNOT").value(KindOfConditional::IfNot),
            parse_directive_word(b"IFDEF").value(KindOfConditional::IfDef),
            parse_directive_word(b"IFNDEF").value(KindOfConditional::IfNdef),
            parse_directive_word(b"IFUSED").value(KindOfConditional::IfUsed),
            parse_directive_word(b"IFEXIST").value(KindOfConditional::IfUsed),
            parse_directive_word(b"IFNUSED").value(KindOfConditional::IfNused)
        ))
        .parse_next(input);

        // leave if the first loop does not have a test
        if first_loop && if_token_or_error.is_err() {
            input.reset(if_start);
            return Err(if_token_or_error.err().unwrap());
        }

        // Get the current condition or nothing for the very last branch
        let condition = if let Ok(test_kind) = if_token_or_error {
            // Get the corresponding test
            let cond = cut_err(
                delimited(my_space0, parse_conditional_condition(test_kind), my_space0)
                    .context("Condition: error in the condition")
            )
            .parse_next(input)?;
            Some(cond)
        }
        else {
            None
        };

        // Remove empty stuff
        let _ = cut_err(
            alt((
                delimited(my_space0, parse_comment, line_ending).recognize(),
                line_ending,
                tag(":")
            ))
            .context("Condition: condition must end by a new line or ':'")
        )
        .parse_next(input)?;

        // get the conditionnal code
        //  dbg!(unsafe{std::str::from_utf8_unchecked(input.as_bytes())});
        let code = cut_err(inner_code.context("Condition: syntax error in conditionnal code"))
            .parse_next(input)?;
        //  dbg!(unsafe{std::str::from_utf8_unchecked(input.as_bytes())});

        if let Some(condition) = condition {
            conditions.push((condition, code));

            let r#else = opt(preceded(
                my_many0_nocollect(alt((
                    my_space1.value(()),
                    line_ending.value(()),
                    tag(":").value(())
                ))),
                parse_directive_word(b"ELSE")
            ))
            .parse_next(input)?;
            if r#else.is_none() {
                break;
            }
        }
        else {
            else_clause = Some(code);
            break;
        }
    }

    // Here we have read the latest block
    // dbg!(unsafe{std::str::from_utf8_unchecked(input.as_bytes())});

    let _ = (
        opt(alt((
            delimited(my_space0, tag(":"), my_space0),
            delimited(my_space0, parse_comment, line_ending).recognize()
        ))),
        cut_err(preceded(my_space0, parse_directive_word(b"ENDIF"))).recognize()
    )
        .context("Condition: end condition not found")
        .parse_next(input)?;

    // dbg!(unsafe{std::str::from_utf8_unchecked(input.as_bytes())}); // endif must have been eaten

    let token = LocatedTokenInner::If(conditions, else_clause)
        .into_located_token_between(if_start, input.clone());
    Ok(token)
}

/// Read the condition part in the parse_conditional macro
#[inline]
fn parse_conditional_condition(
    code: KindOfConditional
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTestKind, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTestKind, Z80ParserError> {
        match &code {
            KindOfConditional::If => {
                located_expr
                    .map(|e| LocatedTestKind::True(e))
                    .parse_next(input)
            },

            KindOfConditional::IfNot => {
                located_expr
                    .map(|e| LocatedTestKind::False(e))
                    .parse_next(input)
            },

            KindOfConditional::IfDef => {
                preceded(space0, parse_label(false))
                    .map(|l| LocatedTestKind::LabelExists(l.into()))
                    .parse_next(input)
            },

            KindOfConditional::IfNdef => {
                parse_label(false)
                    .map(|l| LocatedTestKind::LabelDoesNotExist(l.into()))
                    .parse_next(input)
            },

            KindOfConditional::IfUsed => {
                parse_label(false)
                    .map(|l| LocatedTestKind::LabelUsed(l.into()))
                    .parse_next(input)
            },

            KindOfConditional::IfNused => {
                parse_label(false)
                    .map(|l| LocatedTestKind::LabelNused(l.into()))
                    .parse_next(input)
            },

            _ => unreachable!()
        }
    }
}

/// Parse a breakpint instruction
pub fn parse_breakpoint(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    opt(located_expr)
        .map(|exp| LocatedTokenInner::Breakpoint(exp))
        .parse_next(input)
}

pub fn parse_bankset(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let count = located_expr(input)?;

    Ok(LocatedTokenInner::Bankset(count))
}

pub fn parse_buildsna(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"BUILDSNA").parse_next(input)?;
        }

        terminated(
            cut_err(opt(alt((
                tag_no_case("V2").value(SnapshotVersion::V2),
                tag_no_case("V3").value(SnapshotVersion::V3)
            ))))
            .map(|v: Option<SnapshotVersion>| LocatedTokenInner::BuildSna(v)),
            not(alphanumeric1)
        )
        .parse_next(input)
    }
}


#[derive(PartialEq)]
enum RunEnt {
    Run,
    Ent
}

#[inline]
pub fn parse_run(kind: RunEnt) -> impl Parser<InnerZ80Span, LocatedTokenInner, Z80ParserError> {

    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
    let exp = cut_err(
        located_expr
            .context(
                match &kind {
                RunEnt::Run => "RUN expects at least one expression (e.g. RUN $)",
                RunEnt::Ent => "ENT expects one expression"
            })
        )
        .parse_next(input)?;


    let ga = if kind == RunEnt::Ent {
        opt(preceded((space0, (','), space0), located_expr)).parse_next(input)?
    } else {
        None
    };

    Ok(LocatedTokenInner::Run(exp, ga))
}
}

macro_rules! directive_with_expr {
    ($name:ident, $enum:tt) => {
        #[inline]
        pub fn $name(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
            let exp = located_expr(input)?;

            Ok((LocatedTokenInner::$enum(exp)))
        }
    };
}

directive_with_expr!(parse_map, Map);
directive_with_expr!(parse_limit, Limit);
directive_with_expr!(parse_waitnops, WaitNops);
directive_with_expr!(parse_return, Return);

/// Parse tickin directives
pub fn parse_stable_ticker(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    alt((parse_stable_ticker_start, parse_stable_ticker_stop)).parse_next(input)
}

/// Parse begining of ticker
#[inline]
pub fn parse_stable_ticker_start(
    input: &mut InnerZ80Span
) -> PResult<LocatedTokenInner, Z80ParserError> {
    preceded((tag_no_case("start"), space1), parse_label(false))
        .map(|name| {
            LocatedTokenInner::StableTicker(StableTickerAction::<Z80Span>::Start(name.into()))
        })
        .parse_next(input)
}

/// Parse end of ticker
#[inline]
pub fn parse_stable_ticker_stop(
    input: &mut InnerZ80Span
) -> PResult<LocatedTokenInner, Z80ParserError> {
    tag_no_case("stop")
        .map(|_| LocatedTokenInner::StableTicker(StableTickerAction::Stop))
        .parse_next(input)
}

#[inline]
pub fn parse_bank(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let count = opt(located_expr).parse_next(input)?;

    Ok(LocatedTokenInner::Bank(count))
}

/// Parse fake and real LD instructions
#[inline]
pub fn parse_ld(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        alt((
            parse_ld_fake(mnemonic_name_parsed),
            parse_ld_normal(mnemonic_name_parsed)
        ))
        .parse_next(input)
    }
}
/// Parse artifical LD instruction (would be replaced by several real instructions)
#[inline]
pub fn parse_ld_fake(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if !mnemonic_name_parsed {
            terminated(parse_word(b"LD"), my_space1).parse_next(input)?;
        }

        let dst = alt((
            terminated(
                alt((parse_register16, parse_indexregister16)),
                not(alt((tag_no_case(".low"), tag_no_case(".high"))))
            ),
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        let _ = parse_comma(input)?;

        // TODO - add https://z00m128.github.io/sjasmplus/documentation.html#s_fake_instructions
        let src = if dst.is_register16() {
            alt((
                terminated(
                    alt((parse_register16, parse_indexregister16)),
                    not(alt((tag_no_case(".low"), tag_no_case(".high"))))
                ),
                parse_hl_address,
                parse_indexregister_with_index
            ))
            .parse_next(input)?
        }
        else
        // mem-like
        {
            terminated(
                parse_register16,
                not(alt((tag_no_case(".low"), tag_no_case(".high"))))
            )
            .parse_next(input)?
        };

        let token = LocatedTokenInner::new_opcode(Mnemonic::Ld, Some(dst), Some(src));

        let warning = LocatedTokenInner::WarningWrapper(
            Box::new(token),
            "This is a fake instruction assembled using several opcodes".into()
        );

        Ok(warning)
    }
}
/// Parse the valids LD versions
#[inline]
pub fn parse_ld_normal(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if !mnemonic_name_parsed {
            parse_word(b"LD").parse_next(input)?;
        }

        let _start = input.clone();
        let dst = cut_err(
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
            .context(LD_WRONG_DESTINATION)
        )
        .parse_next(input)?;

        let _ = cut_err(parse_comma.context("LD: missing comma")).parse_next(input)?;

        // src possibilities depend on dst
        let src = cut_err(cut_err(parse_ld_normal_src(&dst)))
            .context(LD_WRONG_SOURCE)
            .parse_next(input)?;

        let token = LocatedTokenInner::new_opcode(Mnemonic::Ld, Some(dst), Some(src));

        Ok(token)
    }
}
/// Parse the source of LD depending on its destination
#[inline]
fn parse_ld_normal_src(
    dst: &LocatedDataAccess
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> + '_ {
    move |input: &mut InnerZ80Span| {
        let input_start = input.checkpoint();
        if dst.is_register_sp() {
            alt((
                parse_register_hl,
                parse_indexregister16,
                parse_address,
                parse_expr
            ))
            .parse_next(input)
        }
        else if dst.is_address_in_register16() || dst.is_address_in_indexregister16() {
            // by construction is t is HL/IX/IY
            alt((parse_register8, parse_expr)).parse_next(input)
        }
        else if dst.is_register16() | dst.is_indexregister16() {
            alt((parse_address, parse_expr)).parse_next(input)
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
                ))
                .parse_next(input)
            }
            else {
                alt((
                    parse_indexregister_address,
                    parse_indexregister_with_index,
                    parse_hl_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8.verify(|src| {
                        if (dst.is_register_h() || dst.is_register_l())
                            && (src.is_register_ixl()
                                || src.is_register_ixh()
                                || src.is_register_ixl()
                                || src.is_register_ixh())
                        {
                            return false;
                        }
                        else {
                            return true;
                        }
                    }),
                    parse_expr
                ))
                .parse_next(input)
            }
        }
        else if dst.is_indexregister8() {
            alt((
                parse_indexregister_address,
                parse_indexregister_with_index,
                parse_hl_address,
                parse_address,
                parse_register8,
                (alt((parse_register_ixh, parse_register_ixl))
                    .verify(|_| dst.is_register_ixl() || dst.is_register_ixh())),
                (alt((parse_register_iyh, parse_register_iyl))
                    .verify(|_| dst.is_register_iyl() || dst.is_register_iyh())),
                parse_expr
            ))
            .parse_next(input)
        }
        else if dst.is_memory() {
            alt((
                parse_register16,
                parse_register8,
                parse_register_sp,
                parse_indexregister16
            ))
            .parse_next(input)
        }
        else if dst.is_address_in_register16() {
            parse_register8(input)
        }
        else if dst.is_indexregister_with_index() {
            alt((parse_register8, parse_expr)).parse_next(input)
        }
        else if dst.is_register_i() || dst.is_register_r() {
            parse_register_a(input)
        }
        else {
            input.reset(input_start);
            Err(ErrMode::Backtrack(
                Z80ParserError::from_error_kind(input, ErrorKind::Alt).into()
            ))
        }
    }
}

/// Parse RES, SET and BIT instructions
#[inline]
pub fn parse_res_set_bit(
    res_or_set: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let bit = cut_err(parse_expr.context("Wrong bit definition")).parse_next(input)?;

        let _ = cut_err(parse_comma).parse_next(input)?;

        let operand = cut_err(
            alt((
                parse_register8,
                parse_hl_address,
                parse_indexregister_with_index
            ))
            .context("Wrong destination")
        )
        .parse_next(input)?;

        // Bit and Res can copy the result in a reg
        let hidden_arg = if res_or_set == Mnemonic::Bit {
            None
        }
        else {
            opt(preceded(parse_comma, parse_register8)).parse_next(input)?
        };

        Ok(LocatedTokenInner::OpCode(
            res_or_set,
            Some(bit),
            Some(operand),
            hidden_arg.map(|d| d.get_register8().unwrap())
        ))
    }
}

/// Parse CP tokens
#[inline]
pub fn parse_cp(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    //   preceded(
    //    parse_word(b"CP"),
    alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr
    ))
    .map(
        //   )
        |operand| LocatedTokenInner::new_opcode(Mnemonic::Cp, Some(operand), None)
    )
    .parse_next(input)
}

#[derive(PartialEq)]
pub enum ExportKind {
    Export,
    NoExport
}

#[inline]
pub fn parse_export(
    code: ExportKind
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let labels: Vec<InnerZ80Span> =
            cut_err(separated0(parse_label(false), parse_comma).context("Wrong parameters"))
                .parse_next(input)?;
        let labels = labels.into_iter().map(|l| Z80Span::from(l)).collect_vec();

        if code == ExportKind::Export {
            Ok(LocatedTokenInner::Export(labels))
        }
        else {
            Ok(LocatedTokenInner::NoExport(labels))
        }
    }
}

#[derive(PartialEq)]
pub enum DbDwStr {
    Db,
    Dw,
    Str
}

#[inline]
/// Parse DB DW directives
pub fn parse_db_or_dw_or_str(
    code: DbDwStr,
    empty_list_allowed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        // STRUCT directive allows to have no arguments
        let expr = if empty_list_allowed {
            expr_list.parse_next(input).unwrap_or(Default::default())
        }
        else {
            expr_list.context(match code {
                DbDwStr::Dw => "DEFW: error in arguments",
                DbDwStr::Db => "DEFB: error in arguments",
                DbDwStr::Str => "STR: error in arguments"
            }).parse_next(input)?
        };

        Ok(match code {
            DbDwStr::Db => LocatedTokenInner::Defb(expr),
            DbDwStr::Dw => LocatedTokenInner::Defw(expr),
            DbDwStr::Str => LocatedTokenInner::Str(expr)
        })
    }
}

// Fail if we do not read a forbidden keyword
#[inline]
pub fn parse_forbidden_keyword(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let start = input.checkpoint();
    let _ = space0(input)?;
    let name = take_while(1.., ('a'..='z', 'A'..='Z', '0'..='9', '_'..='_'))
        .context("Unable to read directive name")
        .parse_next(input)?;

    let mut end_directive_iter = if input.state.options().dotted_directive {
        DOTTED_END_DIRECTIVE.iter()
    }
    else {
        END_DIRECTIVE.iter()
    };

    let name = input.clone().update_slice(name);

    if !end_directive_iter
        .find(|&&a| a == name.to_ascii_uppercase())
        .is_some()
    {
        input.reset(start);
        return Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
            &name,
            ErrorKind::Verify
        )));
    }

    let _ = space0(input)?;

    Ok(name)
}
pub fn parse_macro_arg(input: &mut InnerZ80Span) -> PResult<LocatedMacroParam, Z80ParserError> {
    let _start_input = input.checkpoint();
    let cloned = input.clone();
    let param = alt((
        delimited(
            (space0, ('[')),
            separated0(parse_macro_arg, ','),
            ((']'), space0)
        )
        .map(|l: Vec<LocatedMacroParam>| {
            LocatedMacroParam::List(
                l.into_iter()
                    .map(|p| Box::new(p.clone()))
                    .collect::<Vec<_>>()
            )
        }),
        delimited(
            space0,
            alt((
                located_expr.recognize(), // TODO handle evaluation or transposition
                string_between_quotes.recognize(),
                my_many0_nocollect(none_of((
                    b' ', b',', b'\r', b'\n', b'\t', b']', b'[', b';', b':'
                )))
                .recognize()
            )), // TODO find a way to give arguments with space
            alt((space0, eof))
        )
        .map(|s| cloned.update_slice(s))
        .map(|s| Z80Span::from(s))
        .map(|s| LocatedMacroParam::Single(s))
    ))
    .parse_next(input)?;

    Ok(param)
}

/// Manage the call of a macro.
#[inline]
pub fn parse_macro_or_struct_call_inner(
    for_struct: bool,
    name: InnerZ80Span
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| {
        let input_start = input.checkpoint();

        my_space0.parse_next(input)?;
        not(':').parse_next(input)?;

        if !ignore_ascii_case_allowed_label(name.as_bstr(), input.state.options().dotted_directive)
        {
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_error_kind(input, ErrorKind::Verify).add_context(
                    input,
                    if for_struct {
                        "STRUCT: forbidden name"
                    }
                    else {
                        "MACRO or STRUCT: forbidden name"
                    }
                )
            ));
        }

        // if uncommented we do not detect (void) !
        // let nothing_after = peek((
        // space0,
        // alt((parse_comment.recognize(), tag(":"), tag("\n")))
        // ))
        // .parse_next(input)
        // .is_ok();

        // if allowed_to_return_a_label && nothing_after {
        // let token = LocatedTokenInner::Label(name.into());
        // let msg = format!("Ambiguous code. Use (void) for macro with no args, (default) for struct with default parameters; avoid labels that do not start at beginning of a line. {} is considered to be a label, not a macro.", String::from_utf8_lossy(name.as_bstr()));
        // let warning = LocatedTokenInner::WarningWrapper(Box::new(token), msg);
        // return Ok(warning.into_located_token_at(name.clone()));
        // }

        let _ = (my_space0, not(parse_comment)).parse_next(input)?;
        let input2 = input.clone();

        let args: Vec<(LocatedMacroParam, &[u8])> = if peek(alt((
            eof::<_, Z80ParserError>.value(()),
            tag("\n").value(()),
            tag(":").value(())
        )))
        .parse_next(input)
        .is_ok()
        {
            vec![]
        }
        else {
            cut_err(
                alt((
                    delimited(my_space0, tag_no_case("(void)"), my_space0)
                        .value(Default::default()),
                    alt((
                        tag_no_case("(void)").value(Vec::new()),
                        separated(
                            1..,
                            alt((
                                parse_macro_arg.with_recognized(),
                                space1
                                    .map(|space: &[u8]| {
                                        let space = input2.clone().update_slice(&space[..0]);
                                        LocatedMacroParam::Single(space.into())
                                        // string of size 0;
                                    })
                                    .with_recognized()
                            )),
                            parse_comma
                        )
                    ))
                ))
                .context(if for_struct {
                    "STRUCT: error in arguments list"
                }
                else {
                    "MACRO or STRUCT: forbidden name"
                })
            )
            .parse_next(input)?
        };

        if args.len() == 1 && args.first().unwrap().0.is_empty() {
            panic!();
        }

        // avoid ambiguate code such as label nop
        if args.len() == 1 {
            let mut arg = input.clone().update_slice(args[0].1);
            if alt((
                parse_word(b"NOP").recognize(),
                parse_opcode_no_arg.recognize()
            ))
            .parse_next(&mut arg)
            .is_ok()
            {
                input.reset(input_start);
                return Err(ErrMode::Cut(
                    Z80ParserError::from_error_kind(input, ErrorKind::Verify).add_context(
                        input,
                        if for_struct {
                            "First argument of STRUCT cannot be an opcode with no argument"
                        }
                        else {
                            "First argument of MACRO or STRUCT cannot be an opcode with no argument"
                        }
                    )
                ));
            }
        }

        let args = args.into_iter().map(|(a, _b)| a).collect_vec();
        Ok(LocatedTokenInner::MacroCall(name.into(), args))
    }
}

#[inline]
/// TODO remove by restore the way to parse the macro name
pub fn parse_macro_or_struct_call(
    allowed_to_return_a_label: bool,
    for_struct: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedToken, Z80ParserError> {
        dbg!("here");

        my_space0(input)?;
        let input_start = input.checkpoint();
        let name = terminated(
            parse_macro_name,
            not(alt((
                (space0, alt((tag(":"), line_ending, eof))).recognize(),
                ('.').recognize()
            )))
        )
        .parse_next(input)?;

        // Check if the macro name is allowed
        if !ignore_ascii_case_allowed_label(name.as_bstr(), input.state.options().dotted_directive)
        {
            input.reset(input_start);
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_error_kind(input, ErrorKind::Verify).add_context(
                    input,
                    if for_struct {
                        "STRUCT: forbidden name"
                    }
                    else {
                        "MACRO or STRUCT: forbidden name"
                    }
                )
            ));
        }

        let inner = parse_macro_or_struct_call_inner(for_struct, name).parse_next(input)?;
        let inner = inner.into_located_token_between(input_start, input.clone());
        Ok(inner)
    }
}

/// Manage the call of a macro.
/// When ambiguou may return a label
#[inline]
/// TODO remove by restore the way to parse the macro name
pub fn parse_macro_or_struct_call_beckup_to_remove(
    allowed_to_return_a_label: bool,
    for_struct: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    move |input: &mut InnerZ80Span| {
        panic!("Should not be called anymore");

        // BUG: added because of parsing issues. Need to find why and remove ot
        my_space0(input)?;
        let input_start = input.checkpoint();
        let name = terminated(
            parse_macro_name,
            not(alt((
                (space0, alt((tag(":"), line_ending, eof))).recognize(),
                ('.').recognize()
            )))
        )
        .parse_next(input)?;

        // Check if the macro name is allowed
        if !ignore_ascii_case_allowed_label(name.as_bstr(), input.state.options().dotted_directive)
        {
            input.reset(input_start);
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_error_kind(input, ErrorKind::Verify).add_context(
                    input,
                    if for_struct {
                        "STRUCT: forbidden name"
                    }
                    else {
                        "MACRO or STRUCT: forbidden name"
                    }
                )
            ));
        }

        let nothing_after = peek((
            space0,
            alt((parse_comment.recognize(), tag(":"), tag("\n")))
        ))
        .parse_next(input)
        .is_ok();

        if allowed_to_return_a_label && nothing_after {
            let token = LocatedTokenInner::Label(name.into());
            let msg = format!("Ambiguous code. Use (void) for macro with no args, (default) for struct with default parameters; avoid labels that do not start at beginning of a line. {} is considered to be a label, not a macro.", String::from_utf8_lossy(name.as_bstr()));
            let warning = LocatedTokenInner::WarningWrapper(Box::new(token), msg);
            return Ok(warning.into_located_token_at(name.clone()));
        }

        let _ = (my_space0, not(parse_comment)).parse_next(input)?;
        let input2 = input.clone();

        let args: Vec<(LocatedMacroParam, &[u8])> = if peek(alt((
            eof::<_, Z80ParserError>.value(()),
            tag("\n").value(()),
            tag(":").value(())
        )))
        .parse_next(input)
        .is_ok()
        {
            vec![]
        }
        else {
            cut_err(
                alt((
                    delimited(space0, tag_no_case("(void)"), space0).value(Default::default()),
                    alt((
                        tag_no_case("(void)").value(Vec::new()),
                        separated1(
                            alt((
                                parse_macro_arg.with_recognized(),
                                space1
                                    .map(|space: &[u8]| {
                                        let space = input2.clone().update_slice(&space[..0]);
                                        LocatedMacroParam::Single(space.into())
                                        // string of size 0;
                                    })
                                    .with_recognized()
                            )),
                            parse_comma
                        )
                    ))
                ))
                .context(if for_struct {
                    "STRUCT: error in arguments list"
                }
                else {
                    "MACRO or STRUCT: forbidden name"
                })
            )
            .parse_next(input)?
        };

        if args.len() == 1 && args.first().unwrap().0.is_empty() {
            panic!();
        }

        // avoid ambiguate code such as label nop
        if args.len() == 1 {
            let mut arg = input.clone().update_slice(args[0].1);
            if alt((
                parse_word(b"NOP").recognize(),
                parse_opcode_no_arg.recognize()
            ))
            .parse_next(&mut arg)
            .is_ok()
            {
                input.reset(input_start);
                return Err(ErrMode::Cut(
                    Z80ParserError::from_error_kind(input, ErrorKind::Verify).add_context(
                        input,
                        if for_struct {
                            "First argument of STRUCT cannot be an opcode with no argument"
                        }
                        else {
                            "First argument of MACRO or STRUCT cannot be an opcode with no argument"
                        }
                    )
                ));
            }
        }

        let args = args.into_iter().map(|(a, _b)| a).collect_vec();
        let token = LocatedTokenInner::MacroCall(name.into(), args);
        Ok(token.into_located_token_between(input_start, input.clone()))
    }
}

#[inline]
fn parse_directive_word(
    name: &'static [u8]
) -> impl Fn(&mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> + 'static {
    move |input: &mut InnerZ80Span| {
        if input.state.options().dotted_directive {
            preceded(tag("."), parse_word(name)).parse_next(input)
        }
        else {
            parse_word(name).parse_next(input)
        }
    }
}

#[inline]
/// Consume the word and the empty space after
fn parse_word(
    name: &'static [u8]
) -> impl Fn(&mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<InnerZ80Span, Z80ParserError> {
        let word = terminated(
            tag_no_case(name),
            alt((
                eof.value(()),
                (
                    not(one_of((b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_'))),
                    my_space0
                )
                    .value(())
            ))
        )
        .parse_next(input)?;

        let word = input.clone().update_slice(word);
        Ok(word)
    }
}

/// ...
#[inline]
pub fn parse_djnz(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    preceded(opt(parse_comma), parse_expr)
        .map(|expr| LocatedTokenInner::new_opcode(Mnemonic::Djnz, Some(expr), None))
        .parse_next(input)
}

/// ...
#[inline]
pub fn expr_list(input: &mut InnerZ80Span) -> PResult<Vec<LocatedExpr>, Z80ParserError> {

    let mut exprs = Vec::new();
    loop {
        let expr = opt(located_expr).parse_next(input)?;
        match expr {
            Some(expr) => exprs.push(expr),
            None => break
        }

        let comma = opt(parse_comma).parse_next(input)?;
        match comma {
            Some(_) => {},
            None => break
        }
    }

    Ok(exprs)

}

/// ...
pub fn parse_assert(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let expr = cut_err(located_expr.context("ASSERT: expression error")).parse_next(input)?;

    let exps =
        cut_err(opt(preceded(parse_comma, parse_print_inner)).context("ASSERT: comment error"))
            .parse_next(input)?;

    Ok(LocatedTokenInner::Assert(expr, exps))
}

/// ...
pub fn parse_align(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let boundary = located_expr(input)?;
    let fill = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::Align(boundary, fill))
}

pub fn parse_print_inner(input: &mut InnerZ80Span) -> PResult<Vec<FormattedExpr>, Z80ParserError> {
    separated1(
        alt((
            formatted_expr,
            expr.map(FormattedExpr::from),
            string_between_quotes.map({
                |s: InnerZ80Span| {
                    let s = unsafe { std::str::from_utf8_unchecked(s.as_bstr()) };
                    FormattedExpr::from(Expr::String(SmolStr::from_iter(s.chars())))
                }
            })
        )),
        parse_comma
    )
    .parse_next(input)
}

/// ...
#[inline]
pub fn parse_print(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"PRINT").parse_next(input)?;
        }

        cut_err(parse_print_inner)
            .map(|exps| LocatedTokenInner::Print(exps))
            .parse_next(input)
    }
}

pub fn parse_fail(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"FAIL").parse_next(input)?;
        }

        opt(parse_print_inner)
            .map(|exps| LocatedTokenInner::Fail(exps))
            .parse_next(input)
    }
}

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
#[inline]
fn formatted_expr(input: &mut InnerZ80Span) -> PResult<FormattedExpr, Z80ParserError> {
    let _ = ('{').parse_next(input)?;
    let format = alt((
        tag_no_case("INT").value(ExprFormat::Int),
        tag_no_case("HEX4").value(ExprFormat::Hex(Some(4))),
        tag_no_case("HEX8").value(ExprFormat::Hex(Some(8))),
        tag_no_case("HEX2").value(ExprFormat::Hex(Some(2))),
        tag_no_case("HEX").value(ExprFormat::Hex(None)),
        tag_no_case("BIN8").value(ExprFormat::Bin(Some(8))),
        tag_no_case("BIN16").value(ExprFormat::Bin(Some(16))),
        tag_no_case("BIN32").value(ExprFormat::Bin(Some(32))),
        tag_no_case("BIN").value(ExprFormat::Bin(None))
    ))
    .parse_next(input)?;
    let _ = ('}').parse_next(input)?;

    let _ = space0(input)?;

    let exp = expr(input)?;

    Ok(FormattedExpr::Formatted(format, exp))
}

/// Handle \ in end of line
#[inline]
fn my_space0(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let cloned = input.clone();
    opt(my_space1)
        .recognize()
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

pub fn my_repeat1<I, O, C, E, F>(mut f: F) -> impl Parser<I, C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>
{
    move |i: &mut I| my_repeat1_(&mut f, i)
}

#[inline]
fn my_repeat1_<I, O, C, E, F>(f: &mut F, i: &mut I) -> PResult<C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>
{
    match f.parse_next(i) {
        Err(e) => Err(e.append(i, ErrorKind::Many)),
        Ok(o) => {
            let mut acc = C::initial(None);
            acc.accumulate(o);

            loop {
                let start = i.checkpoint();
                let len = i.eof_offset();
                match f.parse_next(i) {
                    Err(ErrMode::Backtrack(_)) => {
                        i.reset(start);
                        return Ok(acc);
                    },
                    Err(e) => return Err(e),
                    Ok(o) => {
                        // infinite loopmeans eof has been hit
                        if i.eof_offset() == len {
                            return Ok(acc);
                        }

                        acc.accumulate(o);
                    }
                }
            }
        }
    }
}

/// Handle \ in end of line
#[inline]
fn my_space1(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let cloned = input.clone();

    let spaces = alt((
        eof.value(()).context("End of file"), // end of file
        one_of(|c: u8| c.is_space()).value(()).context("Space"), // space char
        (
            // continuated line
            space0,
            '\\',
            space0,
            opt(parse_comment),
            line_ending,
            space0
        )
            .value(())
            .context("continuated line")
    ));

    my_repeat1::<_, _, (), Z80ParserError, _>(spaces)
        .recognize()
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

#[inline]
fn my_line_ending(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let cloned = input.clone();
    alt((line_ending, tag(":")))
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

#[inline]
fn parse_comma(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let cloned = input.clone();
    delimited(my_space0, tag(","), my_space0)
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

/// ...
pub fn parse_protect(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let start = located_expr(input)?;

    let end = preceded(parse_comma, located_expr).parse_next(input)?;

    Ok(LocatedTokenInner::Protect(start, end))
}

#[inline]
/// ...
pub fn parse_logical_operator(
    operator: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let operand = alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr
        ))
        .context("Wrong logical operand")
        .parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(operator, Some(operand), None))
    }
}

/// Substraction with A register
#[inline]
pub fn parse_sub(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =tag_no_case("SUB").parse_next(input)?;
    //  let _ =space1(input)?;

    let _first = opt(terminated(parse_register_a, parse_comma)).parse_next(input)?;

    let operand = alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Sub,
        Some(operand),
        None
    ))
}

/// Par se the SBC instruction
#[inline]
pub fn parse_sbc(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =tag_no_case("SBC").parse_next(input)?;
    //   let _ =space1(input)?;

    let opera = opt(terminated(
        alt((parse_register_a, parse_register_hl)),
        parse_comma
    ))
    .parse_next(input)?;

    let operb = if opera.as_ref().map(|r| r.is_register_a()).unwrap_or(true) {
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr
        ))
        .parse_next(input)
    }
    else {
        alt((parse_register16, parse_register_sp)).parse_next(input)
    }?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Sbc,
        opera,
        Some(operb)
    ))
}

/// Parse ADC and ADD instructions
#[inline]
pub fn parse_add_or_adc(
    add_or_adc: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let first = opt(terminated(
            alt((parse_register_a, parse_register_hl, parse_indexregister16)),
            parse_comma
        ))
        .parse_next(input)?;

        let second = if first.as_ref().map(|f| f.is_register8()).unwrap_or(true) {
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
            .parse_next(input)
        }
        else if first.as_ref().unwrap().is_register16() {
            alt((parse_register16, parse_register_sp)).parse_next(input) // Case for HL XXX AF is accepted whereas it is not the case in real life
        }
        else if first.as_ref().unwrap().is_indexregister16() {
            alt((
                parse_register_bc,
                parse_register_de,
                parse_register_hl,
                parse_register_sp,
                parse_register_ix.verify(|_| first.as_ref().unwrap().is_register_ix()),
                parse_register_iy.verify(|_| first.as_ref().unwrap().is_register_iy())
            ))
            .parse_next(input)
        }
        else {
            return Err(ErrMode::Cut(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Alt
            )));
        }?;

        Ok(LocatedTokenInner::new_opcode(
            add_or_adc,
            first,
            Some(second)
        ))
    }
}

/// ...
#[inline]
pub fn parse_push_n_pop(
    push_or_pop: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let mut registers: Vec<_> =
            separated1(alt((parse_register16, parse_indexregister16)), parse_comma)
                .parse_next(input)?;

        if registers.len() > 1 {
            match push_or_pop {
                Mnemonic::Push => Ok(LocatedTokenInner::MultiPush(registers)),
                Mnemonic::Pop => Ok(LocatedTokenInner::MultiPop(registers)),
                _ => unreachable!()
            }
        }
        else {
            let register = registers.remove(0);
            Ok(LocatedTokenInner::new_opcode(
                push_or_pop,
                Some(register),
                None
            ))
        }
    }
}

/// ...
#[inline]
pub fn parse_ret(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let (cond, cond_bytes) = opt(parse_flag_test).with_recognized().parse_next(input)?;

    let token = LocatedTokenInner::new_opcode(
        Mnemonic::Ret,
        cond.map(|cond| {
            LocatedDataAccess::FlagTest(cond, input.clone().update_slice(cond_bytes).into())
        }),
        None
    );

    Ok(token)
}

/// ...
#[inline]
pub fn parse_inc_dec(
    inc_or_dec: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let register = alt((
            parse_register16,
            parse_indexregister16,
            parse_register8,
            parse_indexregister8,
            parse_register_sp,
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(
            inc_or_dec,
            Some(register),
            None
        ))
    }
}

/// TODO manage other out formats
#[inline]
pub fn parse_out(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =parse_word(b"OUT").parse_next(input)?;

    // get the port proposal
    let port = alt((parse_portc, parse_portnn)).parse_next(input)?;

    // the vlaue depends on the port
    let cloned = input.clone();
    let (value, span) = if port.is_port_c() {
        // reg c
        opt(preceded(
            parse_comma,
            alt((
                parse_register8,
                alt((parse_word(b"f").recognize(), tag("0"))).map(|w| {
                    LocatedDataAccess::Expression(LocatedExpr::Value(
                        0,
                        cloned.update_slice(w).into()
                    ))
                })
            ))
        ))
        .with_recognized()
        .parse_next(input)?
    }
    else {
        preceded(parse_comma, parse_register_a)
            .map(|reg| Some(reg))
            .with_recognized()
            .parse_next(input)?
    };

    let cloned = input.clone();
    let value = value.unwrap_or(LocatedDataAccess::Expression(LocatedExpr::Value(0, {
        cloned.update_slice(span).into()
    })));

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Out,
        Some(port),
        Some(value)
    ))
}

/// Parse all the in flavors
#[inline]
pub fn parse_in(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"IN").parse_next(input)?;
    let cloned = input.clone();
    // get the port proposal
    let (destination, span) = opt(terminated(
        alt((
            parse_register8,
            alt((tag_no_case("f").recognize(), tag("0"))).map(|span| {
                LocatedDataAccess::Expression(LocatedExpr::Value(
                    0,
                    cloned.update_slice(span).into()
                ))
            })
        )),
        parse_comma
    ))
    .with_recognized()
    .parse_next(input)?;

    let cloned = input.clone();
    let destination = destination.unwrap_or(LocatedDataAccess::Expression(LocatedExpr::Value(
        0,
        cloned.update_slice(span).into()
    )));

    let port = cut_err(alt((
        parse_portc,
        parse_portnn.verify(|_| destination.get_register8().unwrap().is_a())
    )))
    .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::In,
        Some(destination),
        Some(port)
    ))
}

/// Parse the rst instruction
#[inline]
pub fn parse_rst(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"RST").parse_next(input)?;
    let val = parse_expr(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Rst,
        Some(val),
        None
    ))
}

/// Parse the IM instruction
#[inline]
pub fn parse_im(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"IM").parse_next(input)?;
    let val = parse_expr(input)?;

    Ok(LocatedTokenInner::new_opcode(Mnemonic::Im, Some(val), None))
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
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let _start = input.clone();
        let arg = alt((
            parse_register8,
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        // hidden opcodes
        let arg2 = opt(preceded(parse_comma, parse_register8)).parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(oper, Some(arg), arg2))
    }
}

/// TODO reduce the flag space for jr"],
#[inline]
pub fn parse_call_jp_or_jr(
    call_jp_or_jr: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        let _start = input.clone();

        let flag_test = opt(terminated(parse_flag_test, parse_comma))
            .with_recognized()
            .parse_next(input)?;

        let dst = cut_err(
            alt((

                    alt((
                        parse_hl_address,
                        parse_indexregister_address,
                        parse_register_hl,
                        parse_indexregister16
                    ))
                .verify(|_| call_jp_or_jr.is_jp() && flag_test.0.is_none()), // not possible for call and for jp/jr when there is flag
                parse_expr
            ))
            .context(match call_jp_or_jr {
                Mnemonic::Jp => JP_WRONG_PARAM,
                Mnemonic::Jr => JR_WRONG_PARAM,
                Mnemonic::Call => CALL_WRONG_PARAM,
                _ => unreachable!()
            })
        )
        .parse_next(input)?;

        // Allow to parse JP HL as to be JP (HL) original notation is misleading
        let dst = match dst {
            LocatedDataAccess::IndexRegister16(reg, span) => {
                LocatedDataAccess::MemoryIndexRegister16(reg, span)
            },
            LocatedDataAccess::Register16(reg, span) => {
                LocatedDataAccess::MemoryRegister16(reg, span)
            },
            other => other
        };

        let flag_test = flag_test.0.map(|f| {
            let span = input.clone().update_slice(flag_test.1);
            LocatedDataAccess::FlagTest(f, span.into())
        });

        Ok(LocatedTokenInner::new_opcode(
            call_jp_or_jr,
            flag_test,
            Some(dst)
        ))
    }
}

/// ...
#[inline]
pub fn parse_flag_test(input: &mut InnerZ80Span) -> PResult<FlagTest, Z80ParserError> {
    alt((
        parse_word(b"NZ").value(FlagTest::NZ),
        parse_word(b"Z").value(FlagTest::Z),
        parse_word(b"NC").value(FlagTest::NC),
        parse_word(b"C").value(FlagTest::C),
        parse_word(b"PO").value(FlagTest::PO),
        parse_word(b"PE").value(FlagTest::PE),
        parse_word(b"P").value(FlagTest::P),
        parse_word(b"M").value(FlagTest::M)
    ))
    .parse_next(input)
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
#[inline]
pub fn parse_register16(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let _start = input.checkpoint();
    let code = terminated(take(2usize), not(alpha1)).parse_next(input)?;

    let reg = match code {
        choice_nocase!(b"AF") => Register16::Af,
        choice_nocase!(b"BC") => Register16::Bc,
        choice_nocase!(b"DE") => Register16::De,
        choice_nocase!(b"HL") => Register16::Hl,
        _ => {
            return Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Alt
            )))
        },
    };

    let span = input.clone().update_slice(code);
    let reg = LocatedDataAccess::Register16(reg, span.into());

    Ok(reg)
}

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
#[inline]
pub fn parse_register8(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    #[derive(PartialEq)]
    enum Reg16Modifier {
        Low,
        High
    };

    alt((
        ((
            parse_register16,
            preceded(
                tag("."),
                alt((
                    tag_no_case("low").map(|_| Reg16Modifier::Low),
                    tag_no_case("high").map(|_| Reg16Modifier::High)
                ))
            ),
            space0
        ))
            .map(|(r16, code, _)| {
                match code {
                    Reg16Modifier::Low => r16.to_data_access_for_low_register().unwrap(),
                    Reg16Modifier::High => r16.to_data_access_for_high_register().unwrap()
                }
            }),
        parse_register_a,
        parse_register_b,
        parse_register_c,
        parse_register_d,
        parse_register_e,
        parse_register_h,
        parse_register_l
    ))
    .parse_next(input)
}

/// Parse register i
#[inline]
pub fn parse_register_i(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let da = ((tag_no_case("I"), not(alphanumeric1)))
        .recognize()
        .parse_next(input)?;
    let da = LocatedDataAccess::SpecialRegisterI(input.clone().update_slice(da).into());
    Ok(da)
}

/// Parse register r
#[inline]
pub fn parse_register_r(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let da = ((tag_no_case("R"), not(alphanumeric1)))
        .recognize()
        .parse_next(input)?;
    let da = LocatedDataAccess::SpecialRegisterR(input.clone().update_slice(da).into());
    Ok(da)
}

macro_rules! parse_any_register8 {
    ($name:ident, $char:expr, $reg:expr) => {
        /// Parse register $char
        #[inline]
        pub fn $name(i: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
            let span = parse_word($char)(i)?;

            Ok((LocatedDataAccess::Register8($reg, span.into())))
        }
    };
}

parse_any_register8!(parse_register_a, b"A", Register8::A);
parse_any_register8!(parse_register_b, b"B", Register8::B);
parse_any_register8!(parse_register_c, b"C", Register8::C);
parse_any_register8!(parse_register_d, b"d", Register8::D);
parse_any_register8!(parse_register_e, b"e", Register8::E);
parse_any_register8!(parse_register_h, b"h", Register8::H);
parse_any_register8!(parse_register_l, b"l", Register8::L);

/// Produce the function that parse a given register
#[inline]
fn register16_parser(
    representation: &'static str,
    register: Register16
) -> impl for<'src, 'ctx> Fn(&mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| {
        let span = ((tag_no_case(representation), not(alphanumeric1)))
            .recognize()
            .parse_next(input)?;

        let span = input.clone().update_slice(span);

        Ok(LocatedDataAccess::Register16(register, span.into()))
    }
}

macro_rules! parse_any_register16 {
    ($name:ident, $char:expr, $reg:expr) => {
        /// Parse the $char register and return it as a DataAccess
        #[inline]
        pub fn $name(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
            register16_parser($char, $reg).parse_next(input)
        }
    };
}

parse_any_register16!(parse_register_sp, "SP", Register16::Sp);
parse_any_register16!(parse_register_af, "AF", Register16::Af);
parse_any_register16!(parse_register_bc, "BC", Register16::Bc);
parse_any_register16!(parse_register_de, "DE", Register16::De);
parse_any_register16!(parse_register_hl, "HL", Register16::Hl);

/// Parse the IX register
#[inline]
pub fn parse_register_ix(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    parse_indexregister16
        .verify(|d: &LocatedDataAccess| d.is_register_ix())
        .parse_next(input)
}

/// Parse the IY register
#[inline]
pub fn parse_register_iy(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    parse_indexregister16
        .verify(|d: &LocatedDataAccess| d.is_register_iy())
        .parse_next(input)
}

// TODO find a way to not use that
macro_rules! parse_any_indexregister8 {
    ($($reg:ident, $alias1:ident, $alias2:ident)*) => {$(
        paste::paste! {
            /// Parse register $reg
            #[inline]
            pub fn [<parse_register_ $reg:lower>] (input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
                let _start = input.clone();
                let span = ((
                    alt((
                        parse_word( stringify!($reg).as_bytes()),
                        parse_word( stringify!($alias1).as_bytes()),
                        parse_word( stringify!($alias2).as_bytes()),
                    ))
                    , not(alphanumeric1)))
                .recognize()
                .parse_next(input)?;

                let span = input.clone().update_slice(span);

                Ok((LocatedDataAccess::IndexRegister8(IndexRegister8::$reg, span.into())))


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
#[inline]
pub fn parse_indexregister8(
    input: &mut InnerZ80Span
) -> PResult<LocatedDataAccess, Z80ParserError> {
    alt((
        parse_register_ixh,
        parse_register_iyh,
        parse_register_ixl,
        parse_register_iyl
    ))
    .parse_next(input)
}

/// Parse a 16 bits indexed register
#[inline]
pub fn parse_indexregister16(
    input: &mut InnerZ80Span
) -> PResult<LocatedDataAccess, Z80ParserError> {
    let code = terminated(take(2usize), not(alpha1))
        .recognize()
        .parse_next(input)?;

    let reg = match code {
        choice_nocase!(b"IX") => IndexRegister16::Ix,
        choice_nocase!(b"IY") => IndexRegister16::Iy,
        _ => {
            return Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Alt
            )))
        },
    };

    let span = input.clone().update_slice(code);
    let reg = LocatedDataAccess::IndexRegister16(reg, span.into());

    Ok(reg)
}

/// Parse the use of an indexed register as (IX + 5)"
#[inline]
pub fn parse_indexregister_with_index(
    input: &mut InnerZ80Span
) -> PResult<LocatedDataAccess, Z80ParserError> {
    let start_checkpoint = input.checkpoint();
    let start_eof_offset = input.eof_offset();
    let (open, _, reg) = ((alt((b'(', b'[')), space0, parse_indexregister16)).parse_next(input)?;

    let op = preceded(
        space0,
        alt((
            b'+'.value(BinaryOperation::Add),
            b'-'.value(BinaryOperation::Sub)
        ))
    )
    .parse_next(input)?;

    let expr = if open == b'(' {
        terminated(located_expr, (space0, b')')).parse_next(input)?
    }
    else {
        assert_eq!(open, b'[');
        terminated(located_expr, (space0, b']'))
            .parse_next(input)
            .unwrap()
    };

    let span = build_span(start_eof_offset, start_checkpoint, input.clone());
    Ok(LocatedDataAccess::IndexRegister16WithIndex(
        reg.get_indexregister16().unwrap(),
        op,
        expr,
        span.into()
    ))
}

/// Parse (C) used in in/out
#[inline]
pub fn parse_portc(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let span = alt((
        ((tag("("), space0, parse_register_c, space0, tag(")"))),
        ((tag("["), space0, parse_register_c, space0, tag("]")))
    ))
    .recognize()
    .parse_next(input)?;
    let span = input.clone().update_slice(span);

    Ok(LocatedDataAccess::PortC(span.into()))
}

/// Parse (nn) used in in/out
#[inline]
pub fn parse_portnn(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let (address, span) = alt((
        delimited(tag("("), located_expr, preceded(space0, tag(")"))),
        delimited(tag("["), located_expr, preceded(space0, tag("]")))
    ))
    .with_recognized()
    .parse_next(input)?;
    let span = input.clone().update_slice(span);

    Ok(LocatedDataAccess::PortN(address, span.into()))
}

/// Parse an address access `(expression)`
#[inline]
pub fn parse_address(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
   /* let filter = |c: u8| {
        c == b'/'
            || c == b'+'
            || c == b'='
            || c == b'-'
            || c == b'*'
            || c == b'<'
            || c == b'>'
            || c == b'%'
            || c == b'&'
            || c == b'|'
    };
    */
    let first_char = alt(('(', '[')).parse_next(input)?;
    let address = terminated(
        located_expr,
        (
            my_space0,
            if first_char == b'(' { b')' } else { b']' },
            peek(
                // filter expressions ; they are followed by some operators
                preceded(my_space0, alt((eof.value(()), my_line_ending.value(()), ",".value(()), ":".value(()))))
            )
        )
    )
    .parse_next(input)?;

    Ok(LocatedDataAccess::Memory(address))
}

/// Parse (R16)
#[inline]
pub fn parse_reg_address(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let (reg, span) = alt((
        delimited(
            terminated(tag("("), space0),
            parse_register16,
            preceded(space0, tag(")"))
        ),
        delimited(
            terminated(tag("["), space0),
            parse_register16,
            preceded(space0, tag("]"))
        )
    ))
    .with_recognized()
    .parse_next(input)?;

    let da = LocatedDataAccess::MemoryRegister16(
        reg.get_register16().unwrap(),
        input.clone().update_slice(span).into()
    );
    Ok(da)
}

/// Parse (HL)
#[inline]
pub fn parse_hl_address(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let span = alt((
        delimited(
            terminated(tag("("), space0),
            parse_register_hl,
            preceded(space0, tag(")"))
        ),
        delimited(
            terminated(tag("["), space0),
            parse_register_hl,
            preceded(space0, tag("]"))
        )
    ))
    .recognize()
    .parse_next(input)?;

    Ok(LocatedDataAccess::MemoryRegister16(
        Register16::Hl,
        input.clone().update_slice(span).into()
    ))
}

/// Parse (ix) and (iy)
#[inline]
pub fn parse_indexregister_address(
    input: &mut InnerZ80Span
) -> PResult<LocatedDataAccess, Z80ParserError> {
    let (reg, res) = delimited(
        terminated(tag("("), space0),
        parse_indexregister16,
        preceded(space0, tag(")"))
    )
    .with_recognized()
    .parse_next(input)?;

    let span = input.clone().update_slice(res);
    Ok(LocatedDataAccess::MemoryIndexRegister16(
        reg.get_indexregister16().unwrap(),
        span.into()
    ))
}

/// Parse an expression and returns it inside a DataAccession::Expression
#[inline]
pub fn parse_expr(input: &mut InnerZ80Span) -> PResult<LocatedDataAccess, Z80ParserError> {
    let expr = located_expr(input)?;
    Ok(LocatedDataAccess::Expression(expr))
}

/// Parse standard org directive
pub fn parse_org(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let val1 = cut_err(located_expr.context("Invalid argument")).parse_next(input)?;
    let val2 = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::Org { val1, val2 })
}

/// Parse defs instruction. TODO add optional parameters
pub fn parse_defs(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let val = separated1(
        cut_err(
            ((located_expr, opt(preceded(parse_comma, located_expr)))).context("Wrong argument")
        ),
        parse_comma
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Defs(val))
}

pub fn parse_nop(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let val = cut_err(
        opt(located_expr.map(|le| LocatedDataAccess::from(le)))
            .context("Wrong argument. NOP expects an expression")
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::OpCode(Mnemonic::Nop, val, None, None))
}

/// Parse any opcode having no argument
pub fn parse_opcode_no_arg(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let cloned = input.clone();
    let token = preceded(
        space0,
        alpha1.verify_map(|word: &[u8]| {
            match word {
                choice_nocase!(b"CCF") => Some(Mnemonic::Ccf),
                choice_nocase!(b"CPD") => Some(Mnemonic::Cpd),
                choice_nocase!(b"CPDR") => Some(Mnemonic::Cpdr),
                choice_nocase!(b"CPI") => Some(Mnemonic::Cpi),
                choice_nocase!(b"CPIR") => Some(Mnemonic::Cpir),
                choice_nocase!(b"CPL") => Some(Mnemonic::Cpl),
                choice_nocase!(b"DAA") => Some(Mnemonic::Daa),
                choice_nocase!(b"DI") => Some(Mnemonic::Di),
                choice_nocase!(b"EI") => Some(Mnemonic::Ei),
                choice_nocase!(b"EXX") => Some(Mnemonic::Exx),
                choice_nocase!(b"HALT") => Some(Mnemonic::Halt),
                choice_nocase!(b"IND") => Some(Mnemonic::Ind),
                choice_nocase!(b"INDR") => Some(Mnemonic::Indr),
                choice_nocase!(b"INI") => Some(Mnemonic::Ini),
                choice_nocase!(b"INIR") => Some(Mnemonic::Inir),
                choice_nocase!(b"LDD") => Some(Mnemonic::Ldd),
                choice_nocase!(b"LDDR") => Some(Mnemonic::Lddr),
                choice_nocase!(b"LDI") => Some(Mnemonic::Ldi),
                choice_nocase!(b"LDIR") => Some(Mnemonic::Ldir),
                choice_nocase!(b"NEG") => Some(Mnemonic::Neg),
                choice_nocase!(b"NOPS2") => Some(Mnemonic::Nop2),
                choice_nocase!(b"OTDR") => Some(Mnemonic::Otdr),
                choice_nocase!(b"OTIR") => Some(Mnemonic::Otir),
                choice_nocase!(b"OUTD") => Some(Mnemonic::Outd),
                choice_nocase!(b"OUTDR") => Some(Mnemonic::Otdr),
                choice_nocase!(b"OUTI") => Some(Mnemonic::Outi),
                choice_nocase!(b"OUTIR") => Some(Mnemonic::Otir),
                choice_nocase!(b"RETI") => Some(Mnemonic::Reti),
                choice_nocase!(b"RETN") => Some(Mnemonic::Retn),
                choice_nocase!(b"RLA") => Some(Mnemonic::Rla),
                choice_nocase!(b"RLCA") => Some(Mnemonic::Rlca),
                choice_nocase!(b"RLD") => Some(Mnemonic::Rld),
                choice_nocase!(b"RRA") => Some(Mnemonic::Rra),
                choice_nocase!(b"RRCA") => Some(Mnemonic::Rrca),
                choice_nocase!(b"RRD") => Some(Mnemonic::Rrd),
                choice_nocase!(b"SCF") => Some(Mnemonic::Scf),
                _ => None
            }
        })
    )
    .with_recognized()
    .map(|(mne, span)| {
        let span = cloned.update_slice(span);
        LocatedTokenInner::OpCode(mne, None, None, None).into_located_token_at(span)
    })
    .parse_next(input)?;

    Ok(token)
}

fn parse_snainit(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let fname = parse_fname(input)?;

    Ok(LocatedTokenInner::SnaInit(fname.into()))
}

fn parse_struct(input: &mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    let name = cut_err(parse_label(false)).parse_next(input)?;

    // TODO parse inner with filtering on the allowed operations
    // would be easier to write and would allow conditional operations
    let fields: Vec<(Z80Span, LocatedToken)> = cut_err(
        repeat(
            1..,
            delimited(
                my_many0_nocollect(alt((
                    space1,
                    parse_comment.recognize(),
                    line_ending,
                    tag(":")
                ))),
                (
                    terminated(
                        parse_label(false),
                        alt((((space0, (':'), space0)).recognize(), space1.recognize()))
                    )
                    .verify(|label: &InnerZ80Span| !label.eq_ignore_ascii_case(b"endstruct"))
                    .context("STRUCT: label error")
                    .map(|span: InnerZ80Span| Z80Span::from(span)),
                    cut_err(parse_struct_directive.context("STRUCT: Invalid operation"))
                ),
                my_many0_nocollect(alt((
                    space1,
                    parse_comment.recognize(),
                    line_ending,
                    tag(":")
                )))
            )
        )
        .context("STRUCT: error in inner content")
    )
    .parse_next(input)?;

    let _ = cut_err(preceded(
        space0,
        alt((
            parse_directive_word(b"ENDSTRUCT"),
            parse_directive_word(b"ENDS")
        ))
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Struct(name.into(), fields))
}

#[inline]
fn parse_snaset(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> PResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"SNASET").parse_next(input)?;
        }

        let input_start = input.checkpoint();
        let flagname = cut_err(parse_label(false).context(SNASET_WRONG_LABEL)).parse_next(input)?;
        let _ = cut_err(parse_comma.context(SNASET_MISSING_COMMA)).parse_next(input)?;

        let values: Vec<_> = cut_err(separated(
            1..,
            parse_flag_value_inner.context("SNASET: wrong flag value"),
            delimited(space0, parse_comma, space0)
        ))
        .parse_next(input)?;

        let flagname = flagname.as_bstr();
        let flagname = unsafe { std::str::from_utf8_unchecked(flagname) };
        let (flagname, value) = if values.len() == 1 {
            (Cow::Borrowed(flagname), values[0].clone())
        }
        else {
            (
                Cow::Owned(format!("{}:{}", flagname, values[0].as_u16().unwrap())),
                values[1].clone()
            )
        };

        let flag = parse_flag::<_, ()>(&mut flagname.as_bytes()).map_err(|_e| {
            input.reset(input_start);
            ErrMode::Backtrack(Z80ParserError::from_error_kind(input, ErrorKind::Verify))
        })?;
        Ok(LocatedTokenInner::SnaSet(flag, value))
    }
}

/// Parse a comment that start by `;` and ends at the end of the line.
#[inline]
pub fn parse_comment(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let cloned = input.clone();
    preceded(alt((b";", b"////")), take_till0(|ch| ch == b'\n'))
        .map(|string: &[u8]| {
            LocatedTokenInner::Comment(cloned.update_slice(string).into())
                .into_located_token_direct()
        })
        .parse_next(input)
}

/// Usefull later for db
#[inline]
pub fn string_between_quotes(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    parse_string(input)
}

/// TODO
#[inline]
pub fn string_expr(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    string_between_quotes
        .map(|string| LocatedExpr::String(string.into()))
        .parse_next(input)
}

#[inline]
pub fn char_expr(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let c = alt((
        delimited(tag("\""), winnow::token::any, tag("\"")),
        delimited(tag("'"), winnow::token::any, tag("'"))
    ))
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::Char(c as char, span.into()))
}

/// Parse a label(label: S)
/// TODO reimplement to never build a string
#[inline]
pub fn parse_label(
    doubledots: bool
) -> impl Fn(&mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| {
        let start = input.checkpoint();


        // Finger crosses that no allocation is done there
        let obtained_label = ((
            opt(alt(("::", "@", "."))).value(()),
            alt((
                one_of((
                    b'a'..=b'z',
                    b'A'..=b'Z',
                    b'_'
                )).value(()),
                delimited('{', expr, '}').value(())
            )),
            my_many0_nocollect(alt((
                take_while(1..,
                    (b'a'..=b'z',
                    b'A'..=b'Z',
                    b'0'..=b'9',
                    b'_')
                  ).value(()),
                ".".value(()),
                delimited('{', opt(expr), '}').value(())
            )))
        )).recognize()
        .parse_next(input)?;

/*
        // fail to parse a label when it is 100% sure it corresponds to  a macro call
        let (macro_arg) = opt(preceded(space1, tag_no_case("(void)".into()))).parse_next(input)?;
        if macro_arg.is_some() {
            return Err(cpclib_common::nom::ErrMode::Backtrack(error_position!(
                input,
                ErrorKind::OneOf
            )));
        }
*/
        let start_with_double_dots = obtained_label.len() > 2 && &obtained_label[..2] == b"::";
        let true_label = if start_with_double_dots {
            &obtained_label[2..]
        }
        else {
            &obtained_label[..]
        };

        //needed because of AT2
        let input = if doubledots {
            let _ =opt(tag_no_case(":")).parse_next(input)?;
            input
        }
        else {
            input
        };


        // Be sure that ::ld is not considered to be a label
        let label_len = true_label.len();
        if label_len >= MIN_MAX_LABEL_SIZE.0 &&
        label_len <= DOTTED_MIN_MAX_LABEL_SIZE.1 &&
            !ignore_ascii_case_allowed_label( true_label, input.state.options().dotted_directive)  {
            
            input.reset(start);
            Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
                input,
                ErrorKind::Verify
            ).add_context(input, "You cannot use a directive or an instruction as a label")
            ))
        }
        else {
            Ok(input.clone().update_slice(obtained_label).into())
        }
    }
}

#[inline]
fn impossible_names(dotted_directive: bool) -> &'static [&'static [u8]] {
    if dotted_directive {
        &DOTTED_IMPOSSIBLE_NAMES
    }
    else {
        &IMPOSSIBLE_NAMES
    }
}

#[inline]
fn ignore_ascii_case_allowed_label(name: &[u8], dotted_directive: bool) -> bool {
    #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
    let iter = impossible_names(dotted_directive).par_iter();
    #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
    let mut iter = impossible_names(dotted_directive).iter();

    !iter.any(|&content| content.eq_ignore_ascii_case(name))
}

#[inline]
pub fn parse_end_directive(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    if input.state.options().dotted_directive {
        b'.'.parse_next(input)?;
    }

    let keyword =
        take_while(1.., (b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_')).parse_next(input)?;

    if END_DIRECTIVE
        .iter()
        .any(|&val| val.eq_ignore_ascii_case(&keyword))
    {
        Ok(input.clone().update_slice(keyword))
    }
    else {
        Err(ErrMode::Backtrack(Z80ParserError::from_error_kind(
            input,
            ErrorKind::Verify
        )))
    }
}

#[inline]
pub fn parse_macro_name(input: &mut InnerZ80Span) -> PResult<InnerZ80Span, Z80ParserError> {
    let dotted_directive = input.state.options().dotted_directive;

    let name = (
        one_of((b'a'..=b'z', b'A'..=b'Z', b'_')),
        take_while(0.., (b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_')),
        not('{')
    )
        .recognize()
        .verify(move |name: &[u8]| {
            if !ignore_ascii_case_allowed_label(name, dotted_directive) {
                return false;
            }
            else {
                return true;
            }
        })
        .parse_next(input)?;

    Ok(input.clone().update_slice(name))
}

#[inline]
pub fn prefixed_label_expr(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let _ = space0(input)?;
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let prefix = alt((
        tag_no_case("{bank}").value(LabelPrefix::Bank),
        tag_no_case("{page}").value(LabelPrefix::Page),
        tag_no_case("{pageset}").value(LabelPrefix::Pageset)
    ))
    .parse_next(input)?;

    let label = preceded(
        space0,
        alt((parse_label(false).recognize(), tag("$$"), tag("$")))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::PrefixedLabel(
        prefix,
        input.clone().update_slice(label).into(),
        span.into()
    ))
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
#[inline]
pub fn parse_value(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let (val, span) = cpclib_common::parse_value
        .with_recognized()
        .parse_next(input)?;

    let span = input.clone().update_slice(span);
    Ok(LocatedExpr::Value(val as i32, span.into()))
}

/// Parse a repetition counter
#[inline]
pub fn parse_counter(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let cloned = input.clone();
    delimited(
        b'{',
        parse_label(false), // BUG will accept too many cases
        (b'}', not(alphanumeric1))
    )
    .recognize()
    .map(|l| LocatedExpr::Label(cloned.update_slice(l).into()))
    .parse_next(input)
}

/// Read a parenthesed expression
#[inline]
pub fn parens(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let exp = delimited(
        delimited(my_space0, tag("("), my_space0),
        located_expr,
        delimited(my_space0, tag(")"), space0)
    )
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::Paren(Box::new(exp), span.into()))
}

#[inline]
pub fn parse_expr_bracketed_list(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let list = delimited(
        (tag("["), my_space0),
        separated0(located_expr, parse_comma),
        (my_space0, tag("]"))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::List(list, span.into()))
}

#[inline]
pub fn parse_bool_expr(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();
    let bool = alt((
        parse_word(b"true").value(true),
        parse_word(b"false").value(false)
    ))
    .parse_next(input)?;
    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::Bool(bool, span.into()))
}

/// Get a factor
#[inline]
pub fn factor(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let neg = opt(delimited(
        space0,
        alt((tag("!").recognize(), parse_word(b"NOT").recognize())),
        space0
    ))
    .parse_next(input)?;

    let not = opt(delimited(space0, tag("~"), space0)).parse_next(input)?;

    let cloned = input.clone();
    let factor = delimited(
        space0,
        alt((
            prefixed_label_expr,
            parse_expr_bracketed_list,
            // Manage functions
            parse_word(b"RND()").map(|w| LocatedExpr::Rnd(w.into())),
            parse_unary_function_call,
            parse_binary_function_call,
            parse_duration,
            parse_assemble,
            parse_any_function_call,
            // manage values
            alt((positive_number, negative_number)),
            char_expr,
            parse_string.map(|s| LocatedExpr::String(s.into())),
            parse_counter,
            // manage $
            tag("$$").map(|l| LocatedExpr::Label(cloned.update_slice(l).into())),
            tag("$").map(|l| LocatedExpr::Label(cloned.update_slice(l).into())),
            parse_bool_expr,
            // manage labels
            parse_label(false).map(|l| LocatedExpr::Label(l.into())),
            parens
        )),
        space0
    )
    .parse_next(input)?;

    let factor = match neg {
        Some(_) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::Neg,
                Box::new(factor),
                build_span(input_offset, input_start, input.clone()).into()
            )
        },
        None => factor
    };

    let factor = match not {
        Some(_) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::Not,
                Box::new(factor),
                build_span(input_offset, input_start, input.clone()).into()
            )
        },
        None => factor
    };

    Ok(factor)
}

#[inline]
pub fn negative_number(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let v = preceded(tag("-"), positive_number)
        .map(|exp| {
            match exp {
                LocatedExpr::Value(v, _) => -v,
                _ => unreachable!()
            }
        })
        .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::Value(v, span.into()))
}

#[inline]
pub fn positive_number(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let _input_start = input.checkpoint();
    let _input_offset = input.eof_offset();

    terminated(
        parse_value,
        not(one_of((
            b'a'..=b'z',
            b'a'..=b'z',
            b'0'..=b'9',
            b'#',
            b'@',
            b'_'
        )))
    )
    .parse_next(input)
}

#[inline]
pub fn parse_labelprefix(input: &mut InnerZ80Span) -> PResult<LabelPrefix, Z80ParserError> {
    alt((
        tag_no_case("{pageset}").value(LabelPrefix::Pageset),
        tag_no_case("{bank}").value(LabelPrefix::Bank),
        tag_no_case("{page}").value(LabelPrefix::Page)
    ))
    .parse_next(input)
}

#[inline]
fn fold_exprs(
    initial: LocatedExpr,
    remainder: Vec<(BinaryOperation, LocatedExpr)>,
    span: InnerZ80Span
) -> LocatedExpr {
    remainder.into_iter().fold(initial, move |acc, pair| {
        let (oper, expr) = pair;
        LocatedExpr::BinaryOperation(oper, Box::new(acc), Box::new(expr), span.clone().into())
    })
}

/// Compute operations related to * % /
#[inline]
pub fn term(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let initial = factor(input)?;
    let remainder = my_many0(alt((
        parse_oper(factor, "*", BinaryOperation::Mul),
        parse_oper(factor, "%", BinaryOperation::Mod),
        parse_oper(factor, "/", BinaryOperation::Div)
    )))
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(fold_exprs(initial, remainder, span.into()))
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
) -> impl Fn(&mut InnerZ80Span) -> PResult<(BinaryOperation, LocatedExpr), Z80ParserError>
where
    F: Fn(&mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError>
{
    #[inline]
    move |input: &mut InnerZ80Span| {
        let _ = space0(input)?;
        let _ = tag_no_case(pattern).parse_next(input)?;
        let _ = space0(input)?;
        let operation = inner(input)?;

        Ok((symbol, operation))
    }
}
#[inline]
fn parse_bool<F>(
    inner: F,
    pattern: &'static str,
    symbol: BinaryOperation
) -> impl Fn(&mut InnerZ80Span) -> PResult<(BinaryOperation, LocatedExpr), Z80ParserError>
where
    F: Fn(&mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError>
{
    #[inline]
    move |input: &mut InnerZ80Span| {
        let _ = space0(input)?;
        let _ = tag_no_case(pattern).parse_next(input)?;
        let _ = space0(input)?;
        let operation = inner(input)?;

        Ok((symbol, operation))
    }
}

/// Parse an expression
#[inline]
pub fn expr2(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let initial = shift(input)?;
    let remainder = my_many0(alt((
        parse_oper(shift, "<=", BinaryOperation::LowerOrEqual),
        parse_oper(shift, "<", BinaryOperation::StrictlyLower),
        parse_oper(shift, ">=", BinaryOperation::GreaterOrEqual),
        parse_oper(shift, ">", BinaryOperation::StrictlyGreater),
        parse_oper(shift, "==", BinaryOperation::Equal),
        parse_oper(shift, "=", BinaryOperation::Equal),
        parse_oper(shift, "!=", BinaryOperation::Different)
    )))
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(fold_exprs(initial, remainder, span.into()))
}

fn expr(input: &mut InnerZ80Span) -> PResult<Expr, Z80ParserError> {
    located_expr
        .map(|e| e.to_expr().into_owned())
        .parse_next(input)
}

/// TODO replace ALL expr parse by a located version
#[inline]
pub fn located_expr(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let initial = expr2(input)?;
    let remainder = my_many0(alt((
        parse_oper(expr2, "&&", BinaryOperation::BooleanAnd),
        parse_oper(expr2, "||", BinaryOperation::BooleanOr)
    )))
    .parse_next(input)?;
    let span = build_span(input_offset, input_start, input.clone());
    Ok(fold_exprs(initial, remainder, span.into()))
}

/// parse functions with one argument
#[inline]
pub fn parse_unary_function_call(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let (word, exp) = (
        delimited(space0, alpha1, space0),
        delimited((space0, tag("("), space0), located_expr, (space0, tag(")")))
            .context("UNARY function: error in parameters")
    )
        .parse_next(input)?;

    let func = match word {
        choice_nocase!(b"HIGH") | choice_nocase!(b"HI") => Some(UnaryFunction::High),
        choice_nocase!(b"LOW") | choice_nocase!(b"LO") => Some(UnaryFunction::Low),
        choice_nocase!(b"PEEK") | choice_nocase!(b"MEMORY") => Some(UnaryFunction::Memory),
        choice_nocase!(b"FLOOR") => Some(UnaryFunction::Floor),
        choice_nocase!(b"CEIL") => Some(UnaryFunction::Ceil),
        choice_nocase!(b"FRAC") => Some(UnaryFunction::Frac),
        choice_nocase!(b"CHAR") => Some(UnaryFunction::Char),
        choice_nocase!(b"INT") => Some(UnaryFunction::Int),
        choice_nocase!(b"SIN") => Some(UnaryFunction::Sin),
        choice_nocase!(b"COS") => Some(UnaryFunction::Cos),
        choice_nocase!(b"ASIN") => Some(UnaryFunction::ASin),
        choice_nocase!(b"ACOS") => Some(UnaryFunction::ACos),
        choice_nocase!(b"LN") => Some(UnaryFunction::Ln),
        choice_nocase!(b"LOG10") => Some(UnaryFunction::Log10),
        choice_nocase!(b"EXP") => Some(UnaryFunction::Exp),
        choice_nocase!(b"SQRT") => Some(UnaryFunction::Char),
        choice_nocase!(b"ABS") => Some(UnaryFunction::Sqrt),
        _ => None
    };

    let span = build_span(input_offset, input_start, input.clone());
    let word = input.clone().update_slice(word);

    let token = match func {
        Some(func) => LocatedExpr::UnaryFunction(func, Box::new(exp), span.into()),
        None => LocatedExpr::AnyFunction(word.into(), vec![exp], span.into())
    };

    Ok(token)
}

/// parse functions with two arguments
#[inline]
pub fn parse_binary_function_call(
    input: &mut InnerZ80Span
) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let func = alt((
        tag_no_case("MIN").value(BinaryFunction::Min),
        tag_no_case("MAX").value(BinaryFunction::Max),
        tag_no_case("POW").value(BinaryFunction::Pow)
    ))
    .parse_next(input)?;

    let _ = ((space0, tag("("), space0)).parse_next(input)?;

    let arg1 = located_expr(input)?;
    let _ = ((space0, tag(","), space0)).parse_next(input)?;
    let arg2 = located_expr(input)?;

    let _ = ((space0, tag(")"))).parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());

    Ok(LocatedExpr::BinaryFunction(
        func,
        Box::new(arg1),
        Box::new(arg2),
        span.into()
    ))
}

#[inline]
pub fn parse_any_function_call(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let function_name = parse_label(false).parse_next(input)?;
    let arguments = delimited(
        (/* space0, */ tag("("), my_space0),
        separated0(located_expr, parse_comma),
        (my_space0, tag(")"))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, input_start, input.clone());
    Ok(LocatedExpr::AnyFunction(
        function_name.into(),
        arguments,
        span.into()
    ))
}

/// Parser for functions taking into argument a token
#[inline]
pub fn token_function<'a>(
    function_name: &'static str
) -> impl Fn(&mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    #[inline]
    move |input: &mut InnerZ80Span| {
        let _ = ((tag_no_case(function_name), space0, ('('), space0)).parse_next(input)?;

        let token = parse_token(input)?;

        let _ = ((space0, tag(")"))).parse_next(input)?;

        Ok(token)
    }
}

/// Parse the duration function
pub fn parse_duration(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let (token, span) = token_function("duration")
        .with_recognized()
        .parse_next(input)?;

    let span = input.clone().update_slice(span).into();
    Ok(LocatedExpr::UnaryTokenOperation(
        UnaryTokenOperation::Duration,
        Box::new(token),
        span
    ))
}

/// Parse the single opcode assembling function
pub fn parse_assemble(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let (token, span) = token_function("opcode")
        .with_recognized()
        .parse_next(input)?;

    let span = input.clone().update_slice(span).into();
    Ok(LocatedExpr::UnaryTokenOperation(
        UnaryTokenOperation::Opcode,
        Box::new(token),
        span
    ))
}

#[inline]
pub fn shift(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let start = input.checkpoint();
    let start_eof_offset = input.eof_offset();

    let initial = comp(input)?;
    let remainder = my_many0(alt((
        parse_oper(comp, "<<", BinaryOperation::LeftShift),
        parse_oper(comp, ">>", BinaryOperation::RightShift)
    )))
    .parse_next(input)?;

    Ok(fold_exprs(
        initial,
        remainder,
        build_span(start_eof_offset, start, input.clone())
    ))
}

/// Parse operation related to + - & |
#[inline]
pub fn comp(input: &mut InnerZ80Span) -> PResult<LocatedExpr, Z80ParserError> {
    let start = input.checkpoint();
    let start_eof_offset = input.eof_offset();

    let initial = term(input)?;
    let remainder = my_many0(alt((
        parse_oper(term, "+", BinaryOperation::Add),
        parse_oper(term, "-", BinaryOperation::Sub),
        parse_oper(term, "&", BinaryOperation::BinaryAnd), /* TODO check if it works and not compete with && */
        parse_oper(term, "AND", BinaryOperation::BinaryAnd),
        parse_oper(term, "|", BinaryOperation::BinaryAnd), /* TODO check if it works and not compete with || */
        parse_oper(term, "OR", BinaryOperation::BinaryOr),
        parse_oper(term, "^", BinaryOperation::BinaryXor), /* TODO check if it works and not compete with ^^ */
        parse_oper(term, "XOR", BinaryOperation::BinaryXor)
    ))).parse_next(input)?;

    Ok(fold_exprs(
        initial,
        remainder,
        build_span(start_eof_offset, start, input.clone())
    ))
}

// Test are deactivated, API is not enough stabilized and tests are broken
#[cfg(test)]
mod test {
    use std::ops::Deref;

    use cpclib_common::winnow::error::ParseError;

    use super::*;

    #[derive(Debug)]
    struct TestResult<O: std::fmt::Debug> {
        ctx: Box<ParserContext>,
        span: Z80Span,
        res: Result<O, ParseError<InnerZ80Span, Z80ParserError>>
    }

    impl<O: std::fmt::Debug> Deref for TestResult<O> {
        type Target = Result<O, ParseError<InnerZ80Span, Z80ParserError>>;

        fn deref(&self) -> &Self::Target {
            &self.res
        }
    }

    #[derive(Debug)]
    struct TestResultRest<O: std::fmt::Debug> {
        ctx: Box<ParserContext>,
        span: Z80Span,
        res: Result<O, ErrMode<Z80ParserError>>
    }

    impl<O: std::fmt::Debug> Deref for TestResultRest<O> {
        type Target = Result<O, ErrMode<Z80ParserError>>;

        fn deref(&self) -> &Self::Target {
            &self.res
        }
    }

    fn parse_test<O, P: Parser<InnerZ80Span, O, Z80ParserError>>(
        mut parser: P,
        code: &'static str
    ) -> TestResult<O>
    where
        O: std::fmt::Debug
    {
        let (ctx, span) = ctx_and_span(code);
        let res = parser.parse(span.0);
        if let Err(e) = &res {
            let e = e.inner();
            let e = AssemblerError::SyntaxError { error: e.clone() };
            eprintln!("Parse error: {}", e);
        }

        TestResult { ctx, span, res }
    }

    fn parse_test_rest<O, P: Parser<InnerZ80Span, O, Z80ParserError>>(
        mut parser: P,
        code: &'static str,
        next: &str
    ) -> TestResultRest<O>
    where
        O: std::fmt::Debug
    {
        let (ctx, mut span) = ctx_and_span(code);
        let res = parser.parse_next(&mut span.0);
        if let Err(ErrMode::Backtrack(e) | ErrMode::Cut(e)) = &res {
            let e = AssemblerError::SyntaxError { error: e.clone() };
            eprintln!("Parse error: {}", e);
        }
        else {
            assert!(unsafe { std::str::from_utf8_unchecked(span.0.as_bstr()) }
                .trim_start()
                .starts_with(next));
        }

        TestResultRest { ctx, span, res }
    }

    fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
        let ctx = Box::new(
            ParserContextBuilder::default()
                .set_context_name("TEST")
                .build(code)
        );
        let span = Z80Span::new_extra(code, ctx.deref());
        (ctx, span)
    }

    #[test]
    fn test_parse_end_directive() {
        let res = parse_test(parse_end_directive, "endif");
        assert!(res.is_ok());

        let res = parse_test(parse_end_directive, "ENDIF");
        assert!(res.is_ok());
    }

    #[test]
    fn test_parse_directive() {
        let res = parse_test(parse_directive, "nop");
        assert!(res.is_ok());

        let res = parse_test(parse_directive, "ORG 10");
        assert!(res.is_ok());
    }

    #[test]
    fn parse_test_cond() {
        let res = parse_test_rest(
            inner_code,
            " nop
        endif",
            "endif"
        );
        assert!(res.is_ok());
        assert_eq!(res.res.unwrap().len(), 1);

        let res = parse_test_rest(
            inner_code,
            " nop
                else",
            "else"
        );
        assert!(res.is_ok());
        assert_eq!(res.res.unwrap().len(), 1);

        let res = parse_test(parse_conditional_condition(KindOfConditional::If), "THING");
        assert!(res.is_ok());

        let res = parse_test(
            (parse_conditional, line_ending, space1),
            "if THING
                    nop
                    endif
                    "
        );
        assert!(res.is_ok());

        let res = parse_test(
            parse_conditional,
            "if THING
                    nop
                    endif "
        );
        assert!(res.is_ok());

        let res = parse_test(
            parse_conditional,
            "if THING
                    nop
                    else
                    nop
                    endif"
        );
        assert!(res.is_ok());

        let res = parse_test(
            parse_conditional,
            "ifndef THING
                    nop
                    else
                    nop
                    endif"
        );
        assert!(res.is_ok());

        let res = parse_test(
            parse_conditional,
            "if demo_system_music_activated != 0
                    ; XXX Ensure memory is properly set
                    ld bc, 0x7fc2 : out (c), c
                    jp PLY_AKYst_Play
                    else
                    WAIT_CYCLES 64*16
                    ret
                    endif"
        );
        assert!(res.is_ok());

        let res = parse_test(
            parse_conditional,
            "ifndef __DEFINED_DEBUG__
                    __DEFINED_DEBUG__ equ 1
                    endif"
        );
        assert!(res.is_ok());

        let mut r#in = Default::default();
        let res = parse_test(
            parse_z80_line_complete(&mut r#in),
            " ifndef __DEFINED_DEBUG__
                    __DEFINED_DEBUG__ equ 1
                    endif"
        );
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn parse_indexregister8() {
        assert_eq!(
            parse_test(parse_register_ixl, "ixl")
                .res
                .unwrap()
                .to_data_access(),
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert_eq!(
            parse_test(parse_register_ixl, "lx")
                .res
                .unwrap()
                .to_data_access(),
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert!(parse_test(parse_register_iyl, "ixl").is_err());
    }

    #[test]
    fn test_parse_prefix_label() {
        let res = parse_test(parse_labelprefix, "{bank}");
        let res = res.res.unwrap();
        assert_eq!(res, LabelPrefix::Bank);

        let res = parse_test(expr, "{bank}label"); // TODO code that
        let res = res.res.unwrap();
        assert_eq!(res, Expr::PrefixedLabel(LabelPrefix::Bank, "label".into()));
    }

    #[test]
    fn test_parse_expr_format() {
        let res = parse_test(formatted_expr, "{hex} VAL");
        assert!(res.is_ok());
        let res = res.res.unwrap();

        assert_eq!(
            res,
            FormattedExpr::Formatted(ExprFormat::Hex(None), Expr::Label("VAL".into()))
        );
    }

    #[test]
    fn test_undocumented_code() {
        let listing = parse_z80_str(" RLC (IY+2), B").unwrap();
        let token = &listing[0];
        let token = token.as_simple_token().into_owned();
        assert_eq!(
            token,
            Token::OpCode(
                Mnemonic::Rlc,
                Some(DataAccess::IndexRegister16WithIndex(
                    IndexRegister16::Iy,
                    BinaryOperation::Add,
                    2.into()
                )),
                Some(DataAccess::Register8(Register8::B)),
                None
            )
        );

        let listing = parse_z80_str(" RES 5, (IY-2), B").unwrap();
        let token = &listing[0];
        let token = token.as_simple_token().into_owned();
        assert_eq!(
            token,
            Token::OpCode(
                Mnemonic::Res,
                Some(DataAccess::Expression(5.into())),
                Some(DataAccess::IndexRegister16WithIndex(
                    IndexRegister16::Iy,
                    BinaryOperation::Sub,
                    2.into()
                )),
                Some(Register8::B)
            )
        );
    }

    #[test]
    fn test_parse_print() {
        let res = parse_test(parse_print(false), "PRINT VAR");
        let res = res.res.unwrap();
        assert_eq!(
            res,
            LocatedTokenInner::Print(vec![FormattedExpr::Raw(Expr::Label("VAR".into()))])
        );

        let res = parse_test(parse_print(false), "PRINT VAR, VAR");
        let res = res.res.unwrap();
        assert_eq!(
            res,
            LocatedTokenInner::Print(vec![
                FormattedExpr::Raw(Expr::Label("VAR".into())),
                FormattedExpr::Raw(Expr::Label("VAR".into()))
            ])
        );

        let res = parse_test(parse_print(false), "PRINT {hex}VAR");
        let res = res.res.unwrap();
        assert_eq!(
            res,
            LocatedTokenInner::Print(vec![FormattedExpr::Formatted(
                ExprFormat::Hex(None),
                Expr::Label("VAR".into())
            )])
        );

        let res = parse_test(parse_print(false), "PRINT \"hello\"");
        let res = res.res.unwrap();
        assert_eq!(
            res,
            LocatedTokenInner::Print(vec![FormattedExpr::Raw(Expr::String("hello".into()))])
        );
    }

    #[test]
    fn test_standard_repeat() {
        let z80 = std::dbg!(
            "  repeat 5
                        nop
                        endrepeat"
        );
        let res = parse_test(parse_repeat, z80);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn test_parse_address() {
        let res = parse_test(parse_address, "(here)");
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn test_parse_word() {
        let res = parse_test(parse_word(b"SNASET"), "SNASET");
        assert!(res.is_ok(), "{:?}", res);

        let res = parse_test(terminated(parse_word(b"SNASET"), my_space1), "SNASET  ");
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn parser_regression_1() {
        let res = parse_test(parse_ld_normal(false), "ld a, chessboard_file");
        assert!(res.is_ok(), "{:?}", res);
    }
    #[test]
    fn parser_regression_1a() {
        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let code: &'static str = unsafe { std::mem::transmute(code.as_str()) };
        let mut vec = Vec::new();
        let res: TestResult<()> = parse_test(repeat(2, parse_z80_line_complete(&mut vec)), code);
        assert!(res.is_ok(), "{:?}", &res);
    }
    #[test]
    fn parser_regression_1c() {
        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let code: &'static str = unsafe { std::mem::transmute(code.as_str()) };
        let res = parse_z80_str(code);
        assert!(res.is_ok(), "{:?}", &res);
    }
    #[test]
    fn parser_regression_1d() {
        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let code: &'static str = unsafe { std::mem::transmute(code.as_str()) };
        let res = parse_test(inner_code, code);
        assert!(res.is_ok());
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
        let res = parse_test(
            inner_code,
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
"
        );
        assert!(res.is_ok(), "{:?}", &res);
    }
    #[test]
    fn parser_regression_1g() {
        let res = parse_test(
            parse_conditional,
            "if 0
                        .load_chessboard
                        ld de, .load_chessboard2
                        ld a, main_memory_chessboard_extra_file
                        jp .common_part_loading_in_main_memory
                        .load_chessboard2
                        ld de, .load_chessboard2
                        ld a, chessboard_file
                        jp .common_part_loading_in_main_memory

                        endif"
        );
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn parser_regression2() {
        let res = parse_test(parse_assert,"assert (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)");
        assert!(res.is_ok(), "{:?}", &res);
    }

    #[test]
    fn parser_sna() {
        let res = parse_test(parse_buildsna(false), "BUILDSNA");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_buildsna(false), "BUILDSNA V2");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_buildsna(false), "BUILDSNA V3");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_buildsna(false), "BUILDSNA V4");
        assert!(res.is_err(), "{:?}", &res);
    }

    #[test]
    fn test_parse_snaset() {
        let res = parse_test(parse_snaset(false), "SNASET Z80_SP, 0x500");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_snaset(false), "SNASET GA_PAL, 0, 30");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_snaset(false), "SNASET CRTC_REG, 1, 48");
        assert!(res.is_ok(), "{:?}", &res);
    }

    #[test]
    fn test_parse_r16_to_r8() {
        let mut r#in = Vec::new();
        let res = parse_test(parse_z80_line_complete(&mut r#in), " ld a, hl.low");
        assert!(res.is_ok(), "{:?}", &res);
        let _res = res.res.unwrap();

        let res = parse_test(parse_ld_normal(false), "ld bc.low, a");
        assert!(res.is_ok(), "{:?}", &res);
        let res = res.res.unwrap().to_token().into_owned();

        assert_eq!(
            res,
            Token::new_opcode(
                Mnemonic::Ld,
                Some(Register8::C.into()),
                Some(Register8::A.into()),
            )
        );

        r#in.clear();
        let res = parse_test(parse_z80_line_complete(&mut r#in), " ld bc.low, a");
        assert!(res.is_ok(), "{:?}", &res);

        assert_eq!(
            r#in.iter().map(|t| t.to_token().into_owned()).collect_vec(),
            vec![Token::new_opcode(
                Mnemonic::Ld,
                Some(Register8::C.into()),
                Some(Register8::A.into()),
            )]
        );

        r#in.clear();
        let res: TestResult<()> = parse_test(
            repeat(2, parse_z80_line_complete(&mut r#in)),
            "\t\tld  bc.low, a\n\t"
        );
        assert!(res.is_ok(), "{:?}", &res);
    }

    #[test]
    fn test_line() {
        let mut tokens = Vec::new();

        let res = parse_test(parse_line(&mut tokens), " hello   ");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), "  ");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), "  ; comment");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), " : ");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), "hello:world");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), " hello :  world");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), " hello:  set world  ");
        assert!(res.is_ok(), "{:?}", &res);
        tokens.clear();

        let res = parse_test(parse_line(&mut tokens), "data1 SETN data");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line(&mut tokens), "data1 SETN data ; comment");
        assert!(res.is_ok(), "{:?}", &res);
    }

    #[test]
    fn test_parse_line_component() {


        let res = parse_test(parse_line_component, "ld      a,(2 - $b06e) and $ff");
        assert!(res.is_ok(), "{:?}", &res);


        let res = parse_test(parse_line_component, " DJNZ CHECK");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "ld a, d");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "sbc h");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "data1 SETN data");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "data2 next data, 2");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(
            (parse_line_component, space1, parse_comment),
            "data1 SETN data ; comment"
        );
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(
            (parse_line_component, space1, parse_comment),
            "data1 setn data ; comment"
        );
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " IN a,(c)");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " IN (c)");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " IN (c)   ");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " DJNZ label");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "label DJNZ label");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " ");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test((parse_line_component, parse_comment), " ; cxcx");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " \\\n");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test((parse_line_component, "\n"), " \n");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "hello");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, " hello ");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "defb 5, 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "defb 5, 20 ");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "xor a");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "xor a ");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "hello xor a ");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR = 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR <<= 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR EQU 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR SET 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR FIELD 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR # 20");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR NEXT VAR2");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "VAR SETN VAR2");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "LET VAR = 5");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "LET 5");
        assert!(res.is_err(), "{:?}", &res);

        let res = parse_test(parse_line_component, "LET VAR");
        assert!(res.is_err(), "{:?}", &res);

        let res = parse_test(
            parse_line_component,
            "for count, 0, 10, 3
		db {count}
	endfor"
        );
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(
            parse_line_component,
            "for count, 0, 10, 3 : db {count} : endfor"
        );
        assert!(res.is_ok(), "{:?}", &res);



        let res = parse_test(parse_line_component, "FAIL");
        assert!(res.is_ok(), "{:?}", &res);


    }

    #[test]
    fn test_regression_while_cpt() {

        let res = parse_test(parse_line_component, "CPT=CPT+1");
        assert!(res.as_ref().unwrap().1.as_ref().unwrap().is_assign(), "{:?}", &res);
    }

    #[test]
    fn test_parse_label() {
        assert!(dbg!(parse_test(parse_label(false), "CHECK")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "label")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "label.label")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "label{after}")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "{before}label")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "la{inner}bel")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "label{i+5}")).is_ok());
    }

    #[test]
    fn test_parse_macro_call() {
        assert!(dbg!(parse_test(parse_line_component, "empty (void)")).is_ok());

        let res = dbg!(parse_test(
            (parse_line_component, ':', parse_line_component),
            "empty (void):ld a,1"
        ))
        .res
        .unwrap();

        assert!(res.0 .0.is_none());
        assert!(res.0 .1.is_some());
        assert!(res.2 .0.is_none());
        assert!(res.2 .1.is_some());

        assert!(dbg!(parse_test(
            parse_line_component,
            "notempty \"arg1\", \"arg2\""
        ))
        .is_ok());
    }

    #[test]
    fn test_regression_check() {
        let check= "CHECK";


        let (ctx, mut span) = ctx_and_span("CHECK");
        assert!(dbg!(factor.parse_next(&mut span.0)).is_ok());

        assert!(dbg!(parse_test(parse_label(false), check)).is_ok());
        assert!(dbg!(parse_test(factor, check)).is_ok());
    }

    #[test]
    fn test_parse_expr() {
        for code in &["(2 - $b06e) and $ff", "'o'", "'o' + 0x80", "CHECK", "\"\\\" et voila\""] {
            assert!(dbg!(parse_test(parse_expr, code)).is_ok());

            assert!(dbg!(parse_test(expr_list, code)).is_ok());
        }
    }

    // TODO find why this test fails wheras cpclib_common::tests::parse_string succeed. I do not get the differences
    #[test]
    fn test_parse_string() {
        for string in &[
            r#""kjkjhkl""#,
            r#""kjk'jhkl""#,
            r#""kj\"kjhkl""#,
            r#"'kjkjhkl'"#,
            r#"'kjk\\"jhkl'"#,
            r#"'kjkj\'hkl'"#,
            r#""""#,
            r#"''"#,
            r#""fdfd\" et voila""#,
            r#""\" et voila""#
        ] {
            let res = parse_test(parse_string, string);
            assert!(dbg!(&res).is_ok());

            assert_eq!(
                res.res.unwrap().as_bstr(),
                (&string[1..string.len() - 1]).as_bstr()
            );

            assert!(dbg!(parse_test(parse_expr, string)).is_ok());
        }
    }



    #[test]
    fn test_parse_macro() {
        let mut tokens = Vec::new();
        let r#macro = "macro bankm
                call xxx
            endm;";
        tokens.clear();
        assert!(dbg!(parse_test(parse_line(&mut tokens), r#macro)).is_ok());


        let r#macro = "bankm macro
        call xxx
    endm;";
        tokens.clear();
        assert!(dbg!(parse_test(parse_line(&mut tokens), r#macro)).is_ok());

    }

    #[test]
    fn test_expression_list() {
        assert!(dbg!(parse_test(expr_list, "1")).is_ok());
        assert!(dbg!(parse_test(expr_list, "1,2")).is_ok());
        assert!(dbg!(parse_test(expr_list, "1, 2")).is_ok());
        assert!(dbg!(parse_test(expr_list, "1 ,2")).is_ok());
        assert!(dbg!(parse_test(expr_list, "1 , 2")).is_ok());
        assert!(dbg!(parse_test(expr_list, "1,2,")).is_ok());
    }
}
