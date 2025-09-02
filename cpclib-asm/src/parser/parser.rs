#![allow(clippy::cast_lossless)]

use core::str;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::DerefMut;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};

use choice_nocase::choice_nocase;
use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use cpclib_common::smallvec::SmallVec;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::ascii::{Caseless, alpha1, alphanumeric1, line_ending, newline, space0};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, eof, not, opt, peek, preceded, repeat, separated, terminated
};
#[allow(deprecated)]
use cpclib_common::winnow::error::ErrorKind;
use cpclib_common::winnow::error::{AddContext, ErrMode, ParserError, StrContext};
use cpclib_common::winnow::stream::{
    Accumulate, AsBStr, AsBytes, AsChar, Checkpoint, LocatingSlice, Offset, Stream, UpdateSlice
};
use cpclib_common::winnow::token::{none_of, one_of, take, take_till, take_until, take_while};
use cpclib_common::winnow::{self, BStr, ModalResult, Parser, Stateful};
use cpclib_sna::parse::parse_flag;
use cpclib_sna::{
    FlagValue, RemuBreakPointAccessMode, RemuBreakPointRunMode, RemuBreakPointType, SnapshotVersion
};
use cpclib_tokens::ListingElement;
// use crc::*;
use obtained::LocatedTokenInner;

use super::context::*;
use super::obtained::*;
use super::orgams::*;
use super::*;
use crate::parser::parser::winnow::combinator::repeat_till;
use crate::preamble::*;

// const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Z80ParserErrorKind {
    /// Static string added by the `context` function
    Context(StrContext),
    /// Indicates which character was expected by the `char` function
    Char(char),
    /// Error kind given by various nom parsers
    Winnow,
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

impl From<char> for Z80ParserErrorKind {
    fn from(other: char) -> Self {
        Self::Char(other)
    }
}

impl Z80ParserError {
    pub fn from_inner_error(
        input: &InnerZ80Span,
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    ) -> Self {
        Self(vec![(*input, Z80ParserErrorKind::Inner { listing, error })])
    }
}

impl ParserError<InnerZ80Span> for Z80ParserError {
    #[allow(deprecated)]
    fn from_error_kind(input: &InnerZ80Span, kind: ErrorKind) -> Self {
        Self(vec![(*input, Z80ParserErrorKind::Winnow)])
    }

    #[allow(deprecated)]
    fn append(
        mut self,
        input: &InnerZ80Span,
        token_start: &<InnerZ80Span as Stream>::Checkpoint,
        kind: ErrorKind
    ) -> Self {
        self.0.push((*input, Z80ParserErrorKind::Winnow));
        self
    }

    fn assert(input: &InnerZ80Span, _message: &'static str) -> Self {
        #[cfg(debug_assertions)]
        panic!("assert `{_message}` failed at {input:#?}");
        #[cfg(not(debug_assertions))]
        #[allow(deprecated)]
        Self::from_error_kind(input, ErrorKind::Assert)
    }

    fn or(self, other: Self) -> Self {
        other
    }
}

impl AddContext<InnerZ80Span> for Z80ParserError {
    fn add_context(
        mut self,
        input: &InnerZ80Span,
        start: &<InnerZ80Span as Stream>::Checkpoint,
        ctx: &'static str
    ) -> Self {
        self.0
            .push((*input, Z80ParserErrorKind::Context(StrContext::Label(ctx))));
        self
    }
}

impl AddContext<InnerZ80Span, StrContext> for Z80ParserError {
    fn add_context(
        mut self,
        input: &InnerZ80Span,
        start: &<InnerZ80Span as Stream>::Checkpoint,
        ctx: StrContext
    ) -> Self {
        self.0.push((*input, Z80ParserErrorKind::Context(ctx)));
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
    b"ABYTE",
    b"ALIGN",
    b"ASMCONTROL",
    b"ASSERT",
    b"BANK",
    b"BANKSET",
    b"BINCLUDE",
    b"BREAK",
    b"BREAKPOINT",
    b"BUILDCPR",
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
    b"FOR",
    b"DW",
    b"ELSE",
    b"ELSEIF",
    b"ELSEIFDEF",
    b"ELSEIFEXIST",
    b"ELSEIFNDEF",
    b"ELSEIFNOT",
    b"ELSEIFUSED",
    //  b"END",
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
    b"INCLZSA1",
    b"INCLZSA2",
    b"INCAPU",
    b"INCZX0",
    b"INCSHRINKLER",
    b"INCUPKR",
    b"LET",
    b"LIMIT",
    b"LIST",
    b"LZEXO",
    b"LZSA1",
    b"LZSA2",
    b"LZUPKR",
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
    b"STARTINGINDEX",
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
    b"ASMCONTROLENV",
    b"CONFINED",
    b"FUNCTION",
    b"FOR",
    b"IF",
    b"IFDEF",
    b"IFEXIST",
    b"IFNDEF",
    b"IFNOT",
    b"IFUSED",
    b"IFNUSED",
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
    b"LZSHRINKLER",
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
    b"END", // for orgams
    b"ENDASMCONTROLENV",
    b"ENDA",
    b"BREAK",
    b"CASE",
    b"CEND",
    b"DEFAULT",
    b"DEPHASE",
    b"ELSE",
    b"ELSEIF",
    b"ELSEIFDEF",
    b"ELSEIFEXIST",
    b"ELSEIFNDEF",
    b"ELSEIFNOT",
    b"ELSEIFUSED",
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

static _DOTTED_END_DIRECTIVE: LazyLock<Vec<String>> = LazyLock::new(|| {
    END_DIRECTIVE
        .iter()
        .map(|d| format!(".{}", { unsafe { std::str::from_utf8_unchecked(d) } }))
        .collect_vec()
});

// tODO use hash-based structures
static _DOTTED_STAND_ALONE_DIRECTIVE: LazyLock<Vec<String>> = LazyLock::new(|| {
    STAND_ALONE_DIRECTIVE
        .iter()
        .map(|d| format!(".{}", unsafe { std::str::from_utf8_unchecked(d) }))
        .collect_vec()
});

static _DOTTED_START_DIRECTIVE: LazyLock<Vec<String>> = LazyLock::new(|| {
    START_DIRECTIVE
        .iter()
        .map(|d| format!(".{}", { unsafe { std::str::from_utf8_unchecked(d) } }))
        .collect_vec()
});

static DOTTED_STAND_ALONE_DIRECTIVE: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    _DOTTED_STAND_ALONE_DIRECTIVE
        .iter()
        .map(String::as_str)
        .map(str::as_bytes)
        .collect_vec()
});
static DOTTED_START_DIRECTIVE: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    _DOTTED_START_DIRECTIVE
        .iter()
        .map(String::as_str)
        .map(str::as_bytes)
        .collect_vec()
});
static DOTTED_END_DIRECTIVE: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    _DOTTED_END_DIRECTIVE
        .iter()
        .map(String::as_str)
        .map(str::as_bytes)
        .collect_vec()
});

static DOTTED_IMPOSSIBLE_NAMES: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    REGISTERS
        .iter()
        .chain(INSTRUCTIONS)
        .chain(DOTTED_STAND_ALONE_DIRECTIVE.iter())
        .chain(DOTTED_START_DIRECTIVE.iter())
        .chain(DOTTED_END_DIRECTIVE.iter())
        .cloned()
        .collect()
});

static IMPOSSIBLE_NAMES: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    REGISTERS
        .iter()
        .chain(INSTRUCTIONS)
        .chain(STAND_ALONE_DIRECTIVE)
        .chain(START_DIRECTIVE)
        .chain(END_DIRECTIVE)
        .cloned()
        .collect()
});

static IMPOSSIBLE_NAMES_ORGAMS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    REGISTERS
        .iter()
        .chain(INSTRUCTIONS)
        .chain(STAND_ALONE_DIRECTIVE_ORGAMS)
        .chain(START_DIRECTIVE_ORGAMS)
        .chain(END_DIRECTIVE_ORGAMS)
        .cloned()
        .collect()
});

static MIN_MAX_LABEL_SIZE: LazyLock<(usize, usize)> = LazyLock::new(|| {
    DOTTED_IMPOSSIBLE_NAMES
        .iter()
        .map(|l| l.len())
        .minmax()
        .into_option()
        .unwrap()
});
static DOTTED_MIN_MAX_LABEL_SIZE: LazyLock<(usize, usize)> = LazyLock::new(|| {
    DOTTED_IMPOSSIBLE_NAMES
        .iter()
        .map(|l| l.len())
        .minmax()
        .into_option()
        .unwrap()
});

/// Produce the stream of tokens. In case of error, return an explanatory string.
/// In case of success loop over all the tokens in order to expand those that read files
pub fn parse_z80_with_context_builder<S: Into<String>>(
    str: S,
    builder: ParserContextBuilder
) -> Result<LocatedListing, AssemblerError> {
    LocatedListing::new_complete_source(str, builder)
        .map_err(|l| AssemblerError::LocatedListingError(std::sync::Arc::new(l)))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub(crate) fn build_span(
    start_eof_offset: usize,
    start: &<InnerZ80Span as Stream>::Checkpoint,
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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_z80<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_str(code)
}

/// Parse a string and return the corresponding listing
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_z80_str<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_with_context_builder(code, ParserContextBuilder::default())
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn my_many0_nocollect<O, E, F>(mut f: F) -> impl FnMut(&mut InnerZ80Span) -> ModalResult<(), E>
where
    F: Parser<InnerZ80Span, O, E>,
    E: ParserError<InnerZ80Span>
{
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |i: &mut InnerZ80Span| {
        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();

            match f.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(&start);
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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn my_many_till_nocollect<O, P, E, F, G>(
    mut f: F,
    mut g: G
) -> impl FnMut(&mut InnerZ80Span) -> ModalResult<((), P), E>
where
    F: Parser<InnerZ80Span, O, E>,
    G: Parser<InnerZ80Span, P, E>,
    E: ParserError<InnerZ80Span>
{
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |i: &mut InnerZ80Span| {
        loop {
            let start_i = i.checkpoint();
            let len = i.eof_offset();
            match g.parse_next(i) {
                Ok(o) => return Ok(((), o)),
                Err(ErrMode::Backtrack(e)) => {
                    match f.parse_next(i) {
                        Err(ErrMode::Backtrack(_err)) => {
                            i.reset(&start_i);
                            #[allow(deprecated)]
                            return Err(ErrMode::Backtrack(e.append(i, &start_i, ErrorKind::Many)));
                        },
                        Err(e) => return Err(e),
                        Ok(_o) => {
                            // infinite loop check: the parser must always consume
                            if i.eof_offset() == len {
                                return Err(ErrMode::Backtrack(E::from_input(i)));
                            }
                        }
                    }
                },
                Err(e) => return Err(e)
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn inner_code(input: &mut InnerZ80Span) -> ModalResult<LocatedListing, Z80ParserError> {
    inner_code_with_state(input.state.state, false).parse_next(input)
}
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn one_instruction_inner_code(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedListing, Z80ParserError> {
    inner_code_with_state(input.state.state, true).parse_next(input)
}

/// Workaround because many0 is not used in the main root function
/// TODO add an argument to handle context change
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn inner_code_with_state(
    new_state: ParsingState,
    only_one_instruction: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedListing, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        // dbg!("Requested state", &new_state);
        LocatedListing::parse_inner(input, new_state, only_one_instruction)
            .map(|l| Arc::<LocatedListing>::try_unwrap(l).unwrap())
    }
}

/// TODO
pub fn parse_rorg(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let _ = my_space0.parse_next(input)?;
    let rorg_start = input.checkpoint();
    let _ = alt((Caseless("PHASE"), Caseless("RORG"))).parse_next(input)?;

    let exp = cut_err(
        delimited(my_space1, located_expr, my_space0)
            .context(StrContext::Label("RORG: error in the expression"))
    )
    .parse_next(input)?;

    let _ = my_line_ending.parse_next(input)?;

    let inner = inner_code.parse_next(input)?;

    let _ = cut_err(
        preceded(my_space0, alt((Caseless("DEPHASE"), Caseless("REND"))))
            .context(StrContext::Label("RORG: missing REND"))
    )
    .parse_next(input)?;

    let _rorg_stop = input.checkpoint();
    let token = LocatedTokenInner::Rorg(exp, inner).into_located_token_between(&rorg_start, *input);
    Ok(token)
}

/// TODO - limit the listing possibilities
pub fn parse_function_listing(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedListing, Z80ParserError> {
    // dbg!("parse_function_listing requests FunctionLimited state");
    inner_code_with_state(ParsingState::FunctionLimited, false).parse_next(input)
}

pub fn parse_function(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let function_start = input.checkpoint();
    let _ = preceded(my_space0, parse_directive_word(b"FUNCTION")).parse_next(input)?;
    let name = cut_err(parse_label(false).context(StrContext::Label("FUNCTION: wrong name")))
        .parse_next(input)?; // TODO use a specific function for that

    let cloned = *input;
    let arguments: Vec<InnerZ80Span> = cut_err(
        preceded(
            opt(parse_comma), // comma after macro name is not mandatory
            separated::<_, InnerZ80Span, Vec<InnerZ80Span>, _, _, _, _>(
                0..,
                // parse_label(false)
                delimited(
                    my_space0,
                    take_till(1.., |c| {
                        c == b'\n' || c == b'\r' || c == b':' || c == b',' || c == b' '
                    })
                    .map(|s: &[u8]| cloned.update_slice(s)),
                    my_space0
                ),
                parse_comma
            )
        )
        .context(StrContext::Label("FUNCTION: errors in parameters"))
    )
    .parse_next(input)?;
    let arguments = arguments.into_iter().map(|span| span.into()).collect_vec();

    cut_err(
        preceded(my_space0, my_line_ending)
            .context(StrContext::Label("FUNCTION: errors after parameters"))
    )
    .parse_next(input)?;

    let listing =
        cut_err(parse_function_listing.context(StrContext::Label("FUNCTION: invalid content")))
            .parse_next(input)?;

    repeat::<_, _, (), _, _>(0.., my_line_ending).parse_next(input)?;
    let _ = alt((
        parse_directive_word(b"ENDF"),
        parse_directive_word(b"ENDFUNCTION")
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Function(name.into(), arguments, listing)
        .into_located_token_between(&function_start, *input))
}

/// TODO
pub fn parse_macro(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let dir_start = input.checkpoint();
    let _ = parse_directive_word(b"MACRO").parse_next(input)?;

    // macro name
    let name = cut_err(parse_label(false).context(StrContext::Label("MACRO: wrong name")))
        .parse_next(input)?; // TODO use a specific function for that

    parse_macro_inner(dir_start, name).parse_next(input)
}

fn parse_macro_inner(
    dir_start: <InnerZ80Span as Stream>::Checkpoint,
    name: InnerZ80Span
) -> impl FnMut(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedToken, Z80ParserError> {
        #[derive(Clone, Copy, Debug)]
        enum CommaOrParenthesis {
            Comma,
            Parenthesis
        }

        let comma_or_parenthesis = opt(alt((
            parse_comma.value(CommaOrParenthesis::Comma),
            '('.value(CommaOrParenthesis::Parenthesis)
        )))
        .parse_next(input)?;

        // macro arguments
        let arguments = separated::<_, _, Vec<&[u8]>, _, _, _, _>(
            0..,
            // parse_label(false)
            delimited(
                my_space0,
                take_till(1.., |c| {
                    c == b'\n'
                        || c == b'\r'
                        || c == b':'
                        || c == b','
                        || c == b' '
                        || c == b')'
                        || c == b';'
                }),
                my_space0
            ),
            parse_comma
        )
        .parse_next(input)?;

        if let Some(CommaOrParenthesis::Parenthesis) = comma_or_parenthesis {
            cut_err(
                (my_space0, ')', my_space0)
                    .value(())
                    .context("`)` expected`")
            )
            .parse_next(input)?;
        }

        let arguments = arguments
            .into_iter()
            .map(|span| (*input).update_slice(span))
            .map(|span| span.into())
            .collect_vec();

        alt((my_space0.value(()), my_line_ending.value(()))).parse_next(input)?;

        // TODO factorize with the code of parse_basic
        let before_content = input.checkpoint();
        let (_, end) = cut_err(
            repeat_till::<_, _, (), _, _, _, _>(
                0..,
                take(1usize),
                alt((
                    parse_directive_word(b"ENDM"),
                    parse_directive_word(b"ENDMACRO"),
                    parse_directive_word(b"MEND")
                ))
            )
            .context(StrContext::Label(
                "MACRO: impossible to collect macro content"
            ))
        )
        .parse_next(input)?;

        let content_length = end.offset_from(&before_content);
        let mut content = *input;
        content.reset(&before_content);
        let content: &BStr = unsafe { std::mem::transmute(&content.as_bstr()[..content_length]) };
        let content = (*input).update_slice(content); // TODO find a way to improve that part. I'd like to not make the conversion

        Ok(LocatedTokenInner::Macro {
            name: name.into(),
            params: arguments,
            content: content.into(),
            flavor: input.state.options().assembler_flavor
        }
        .into_located_token_between(&dir_start, *input))
    }
}

/// TODO
pub fn parse_while(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let _ = my_space0(input)?;
    let while_start = input.checkpoint();
    let _ = parse_directive_word(b"WHILE").parse_next(input)?;

    let cond = cut_err(located_expr.context(StrContext::Label("WHILE: error in condition")))
        .parse_next(input)?;

    // we must have either a new line or :
    alt((
        delimited(my_space0, ':', my_space0).value(()),
        preceded(my_space0, line_ending).value(())
    ))
    .parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label("WHILE: issue in the content")))
        .parse_next(input)?;
    let _ = cut_err(
        preceded(
            my_space0,
            alt((parse_directive_word(b"ENDW"), parse_directive_word(b"WEND")))
        )
        .context(StrContext::Label("WHILE: not closed"))
    )
    .parse_next(input)?;

    let token =
        LocatedTokenInner::While(cond, inner).into_located_token_between(&while_start, *input);
    Ok(token)
}

pub fn parse_module(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let module_start = input.checkpoint();
    let _ = parse_directive_word(b"MODULE").parse_next(input)?;

    let name = cut_err(parse_label(false).context(StrContext::Label("MODULE: error in naming")))
        .parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label("MODULE: issue in the content")))
        .parse_next(input)?;
    let _ = cut_err(
        preceded(my_space0, parse_directive_word(b"ENDMODULE"))
            .context(StrContext::Label("MODULE: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::Module(name.into(), inner)
        .into_located_token_between(&module_start, *input);
    Ok(token)
}

/// Parse a sub-listing part that aims at being crunched after being assembled at first pass
pub fn parse_crunched_section(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let crunched_start = input.checkpoint();
    let kind = preceded(
        my_space0,
        alt((
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZEXO").value(CrunchType::LZEXO),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZ4").value(CrunchType::LZ4),
            parse_directive_word(b"LZ48").value(CrunchType::LZ48),
            parse_directive_word(b"LZ49").value(CrunchType::LZ49),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZSHRINKLER").value(CrunchType::Shrinkler),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZUPKR").value(CrunchType::Upkr),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZX7").value(CrunchType::LZX7),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZX0").value(CrunchType::LZX0),
            #[cfg(not(target_arch = "wasm32"))]
            parse_directive_word(b"LZAPU").value(CrunchType::LZAPU),
            parse_directive_word(b"LZSA1").value(CrunchType::LZSA1),
            parse_directive_word(b"LZSA2").value(CrunchType::LZSA2)
        ))
    )
    .parse_next(input)?;

    let inner =
        cut_err(inner_code.context(StrContext::Label("CRUNCHED SECTION: issue in the content")))
            .parse_next(input)?;

    let _ = cut_err(
        ((my_space0, parse_directive_word(b"LZCLOSE"), my_space0))
            .context(StrContext::Label("CRUNCHED SECTION section: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::CrunchedSection(kind, inner)
        .into_located_token_between(&crunched_start, *input);
    Ok(token)
}

/// Parse the switch directive
pub fn parse_switch(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    my_many0_nocollect(alt((my_space1.value(()), my_line_ending.value(())))).parse_next(input)?;
    let switch_start = *input;
    let _ = parse_directive_word(b"SWITCH")(input)?;

    let value = cut_err(
        preceded(my_space0, located_expr).context(StrContext::Label("SWITCH: tested value"))
    )
    .parse_next(input)?;

    let mut cases_listing = Vec::new();
    let mut default_listing = None;

    loop {
        cut_err(
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    my_space1.value(()),
                    line_ending.value(()),
                    ':'.value(()),
                    parse_comment.value(())
                ))
            )
            .context(StrContext::Label("SWITCH: whitespace error"))
        )
        .parse_next(input)?;

        // after default it is mandatory to end the block
        let endswitch = if default_listing.is_some() {
            cut_err(
                preceded(
                    my_space0,
                    alt((
                        parse_directive_word(b"ENDS"),
                        parse_directive_word(b"ENDSWITCH")
                    ))
                    .value(true)
                )
                .context(StrContext::Label(
                    "SWITCH: endswitch not present after default listing."
                ))
            )
            .parse_next(input)?
        }
        else {
            preceded(
                my_space0,
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
                .into_located_token_between(&switch_start.checkpoint(), *input);
            return Ok(token);
        }

        let value = preceded(my_space0, opt(parse_directive_word(b"CASE"))).parse_next(input)?;
        if value.is_some() {
            let value = cut_err(
                delimited(my_space0, located_expr, opt(':'))
                    .context(StrContext::Label("SWITCH: case value error."))
            )
            .parse_next(input)?;

            let inner =
                cut_err(inner_code.context(StrContext::Label("SWITCH: error in case code")))
                    .parse_next(input)?;

            let do_break =
                opt(preceded(my_space0, parse_directive_word(b"BREAK"))).parse_next(input)?;

            cases_listing.push((value, inner, do_break.is_some()));
        }
        else {
            let _ = cut_err(
                delimited(
                    my_space0,
                    parse_directive_word(b"DEFAULT"),
                    opt((my_space0, ':'))
                )
                .context(StrContext::Label(
                    "Only CASE, DEFAULT or ENDSWITCH are expected."
                ))
            )
            .parse_next(input)?;
            let default =
                cut_err(inner_code.context(StrContext::Label("SWITCH: error in default case")))
                    .parse_next(input)?;
            default_listing = Some(default);
        }
    }
}

pub fn parse_for(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let for_start = input.checkpoint();
    let _ = preceded(my_space0, parse_directive_word(b"FOR")).parse_next(input)?;

    // Get parameters
    let counter = cut_err(parse_label(false)).parse_next(input)?;
    let start = cut_err(preceded(parse_comma, located_expr)).parse_next(input)?;
    let stop = cut_err(preceded(parse_comma, located_expr)).parse_next(input)?;
    let step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    // Get loop content
    let inner = cut_err(inner_code.context(StrContext::Label("FOR: issue in the content")))
        .parse_next(input)?;

    // Collect end of loop
    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDFOR"),
                parse_directive_word(b"FEND"),
                parse_directive_word(b"ENDF")
            ))
        )
        .context(StrContext::Label("FOR: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::For {
        label: counter.into(),
        start,
        stop,
        step,
        listing: inner
    }
    .into_located_token_between(&for_start, *input);
    Ok(token)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_confined(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    // let _ = my_space0(input)?;
    let confined_start = input.checkpoint();

    let _ = parse_directive_word(b"CONFINED").parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label("CONFINED: issue in the content")))
        .parse_next(input)?;

    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDCONFINED"),
                parse_directive_word(b"CEND"),
                parse_directive_word(b"ENDC")
            ))
        )
        .context(StrContext::Label("CONFINED: not closed"))
    )
    .parse_next(input)?;

    let token =
        LocatedTokenInner::Confined(inner).into_located_token_between(&confined_start, *input);
    Ok(token)
}

pub fn parse_repeat(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let repeat_start = input.checkpoint();
    let _ = preceded(
        my_space0,
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
                    .context(StrContext::Label("REPEAT: issue in the counter"))
            )
            .parse_next(input)?;
            let counter_start = opt(preceded(parse_comma, located_expr)).parse_next(input)?;
            let counter_step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

            let inner =
                cut_err(inner_code.context(StrContext::Label("REPEAT: issue in the content")))
                    .parse_next(input)?;

            let _ = cut_err(
                preceded(
                    my_space0,
                    alt((
                        parse_directive_word(b"ENDREPEAT"),
                        parse_directive_word(b"ENDREPT"),
                        parse_directive_word(b"ENDREP"),
                        parse_directive_word(b"ENDR"),
                        parse_directive_word(b"REND")
                    ))
                )
                .context(StrContext::Label("REPEAT: not closed"))
            )
            .parse_next(input)?;

            let token = LocatedTokenInner::Repeat(
                count,
                inner,
                counter.map(|c| c.into()),
                counter_start,
                counter_step
            )
            .into_located_token_between(&repeat_start, *input);
            Ok(token)
        },

        None => {
            let inner =
                cut_err(inner_code.context(StrContext::Label("REPEAT: issue in the content")))
                    .parse_next(input)?;

            let _ = cut_err(
                delimited(my_space0, parse_directive_word(b"UNTIL"), my_space0)
                    .context(StrContext::Label("REPEAT ... UNTIL: not closed"))
            )
            .parse_next(input)?;
            let cond =
                cut_err(located_expr.context(StrContext::Label("REPEAT UNTIL: condition error")))
                    .parse_next(input)?;
            let token = LocatedTokenInner::RepeatUntil(cond, inner)
                .into_located_token_between(&repeat_start, *input);
            Ok(token)
        }
    }
}

pub fn parse_iterate(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let iterate_start = input.checkpoint();
    let _ = preceded(
        my_space0,
        alt((
            parse_directive_word(b"ITERATE"),
            parse_directive_word(b"ITER")
        ))
    )
    .parse_next(input)?;

    let counter = cut_err(
        preceded(my_space0, parse_label(false))
            .context(StrContext::Label("ITERATE: issue in the counter"))
    )
    .parse_next(input)?;

    let comma_or_in = cut_err(
        preceded(my_space0, alt((parse_word(b"IN"), parse_comma)))
            .context(StrContext::Label("ITERATE: expected ',' or 'in'"))
    )
    .parse_next(input)?;

    let values = if comma_or_in.contains(&b',') {
        let values = cut_err(expr_list.context(StrContext::Label("ITERATE: values issue")))
            .parse_next(input)?;
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
            .context(StrContext::Label("ITERATE: list issue"))
        )
        .parse_next(input)?;
        either::Either::Right(values)
    };

    let inner = cut_err(inner_code.context(StrContext::Label("ITERATE: issue in the content")))
        .parse_next(input)?;

    let _ = cut_err(
        ((
            my_space0,
            alt((
                parse_directive_word(b"ENDITERATE"),
                parse_directive_word(b"ENDITER"),
                parse_directive_word(b"ENDI"),
                parse_directive_word(b"IEND")
            )),
            my_space0
        ))
            .context(StrContext::Label("ITERATE: not closed"))
    )
    .parse_next(input)?;

    let token = LocatedTokenInner::Iterate(counter.into(), values, inner)
        .into_located_token_between(&iterate_start, *input);
    Ok(token)
}

/// TODO
pub fn parse_basic(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let basic_start = input.checkpoint();
    let _ = ((my_space0, Caseless("LOCOMOTIVE"), my_space0)).parse_next(input)?;

    // collect the labels that are spread to the basic environnement
    let args: Option<Vec<InnerZ80Span>> = opt(separated(
        1..,
        preceded(my_space0, parse_label(false)),
        parse_comma
    ))
    .parse_next(input)?;
    let args = args.map(|args| args.into_iter().map(Z80Span::from).collect_vec());

    (my_space0, opt(line_ending)).parse_next(input)?;

    let hidden_lines = opt(terminated(
        preceded(my_space0, parse_basic_hide_lines),
        my_space0
    ))
    .parse_next(input)?;

    (my_space0, opt(line_ending)).parse_next(input)?;

    // TODO factorize with the the code of parse_macro
    let before_content = input.checkpoint();
    let (_, end) = cut_err(
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            take(1usize),
            parse_directive_word(b"ENDLOCOMOTIVE")
        )
        .context(StrContext::Label(
            "BASIC: impossible to collect BASIC content"
        ))
    )
    .parse_next(input)?;

    let content_length = end.offset_from(&before_content);
    let mut content = *input;
    content.reset(&before_content);
    let content: &BStr = unsafe { std::mem::transmute(&content.as_bstr()[..content_length]) };
    let basic = (*input).update_slice(content); // TODO find a way to improve that part. I'd like to not make the conversion

    let _ = my_space0.parse_next(input)?;

    let token = LocatedTokenInner::Basic(args, hidden_lines, basic.into())
        .into_located_token_between(&basic_start, *input);
    Ok(token)
}

/// Parse the instruction to hide basic lines
pub fn parse_basic_hide_lines(
    input: &mut InnerZ80Span
) -> ModalResult<Vec<LocatedExpr>, Z80ParserError> {
    let _ = ((Caseless("HIDE_LINES"), my_space1)).parse_next(input)?;
    expr_list.parse_next(input)
}

pub fn parse_flag_value_inner(input: &mut InnerZ80Span) -> ModalResult<FlagValue, Z80ParserError> {
    let start = input.checkpoint();
    cpclib_sna::parse::parse_flag_value::<InnerZ80Span, Z80ParserError>
        .parse_next(input)
        .map_err(|e| {
            match e {
                ErrMode::Incomplete(_) => todo!(),
                ErrMode::Backtrack(e) => {
                    let mut error = Z80ParserError::from_input(input);
                    for ctx in e.context() {
                        error = error.add_context(input, &start, ctx.clone());
                    }

                    ErrMode::Backtrack(error)
                },
                ErrMode::Cut(e) => {
                    let mut error = Z80ParserError::from_input(input);
                    for ctx in e.context() {
                        error = error.add_context(input, &start, ctx.clone());
                    }

                    ErrMode::Cut(error)
                }
            }
        })
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_line_component(
    input: &mut InnerZ80Span
) -> ModalResult<(Option<LocatedToken>, Option<LocatedToken>), Z80ParserError> {
    my_space0.parse_next(input)?;

    parse_line_component_standard.parse_next(input)
}

/// Optionally return a label and a command
/// next  token is a separator :, \n, eof
pub fn parse_line_component_standard(
    input: &mut InnerZ80Span
) -> ModalResult<(Option<LocatedToken>, Option<LocatedToken>), Z80ParserError> {
    if input.state.options().is_orgams() {
        let repeat = opt(parse_orgams_repeat).parse_next(input)?;
        if repeat.is_some() {
            return Ok((None, repeat));
        }
    }

    let before_let = input.checkpoint();
    let r#let = terminated(opt(parse_directive_word(b"LET")), my_space0).parse_next(input)?;

    let before_label = input.checkpoint();

    let mut label: Option<InnerZ80Span> = if r#let.is_some() {
        // label is mandatory when there is let
        cut_err(
            parse_label(false)
                .context(StrContext::Label("LET: missing label"))
                .map(Some)
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
        cut_err(b"=".context(StrContext::Label("LET: missing =")))
            .map(Some)
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
            terminated(parse_word(b"SET"), not((my_space0, expr, parse_comma)))
                .map(|_| LabelModifier::Set),
            b"=".value(LabelModifier::Equal(None)),
            alt((parse_word(b"FIELD").value(()), b"#".value(()))).value(LabelModifier::Field),
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
            ))
            .map(|oper| LabelModifier::Equal(Some(oper)))
        )))
        .parse_next(input)?
    };

    if let Some(label_modifier) = label_modifier {
        if label_modifier == LabelModifier::Macro {
            let r#macro = parse_macro_inner(before_label, label.unwrap())
                .context(StrContext::Label("MACRO: error on macro definition"))
                .parse_next(input)?;
            return Ok((None, Some(r#macro)));
        }

        let expr_arg = match &label_modifier {
            LabelModifier::Equ
            | LabelModifier::Equal(..)
            | LabelModifier::Set
            | LabelModifier::Field => {
                cut_err(located_expr.map(Some))
                    .context(StrContext::Label("Value error"))
                    .parse_next(input)?
            },
            _ => None
        };

        let source_label = match &label_modifier {
            LabelModifier::Next | LabelModifier::SetN => {
                cut_err(
                    preceded(my_space0, parse_label(false))
                        .map(Some)
                        .context(StrContext::Label("Label expected"))
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
                    expr: expr_arg.unwrap()
                }
            },
            LabelModifier::Macro => unreachable!("This case must have been handled before")
        }
        .into_located_token_between(&before_label, *input);

        Ok((None, Some(token)))
    }
    else {
        // ensure we have not eaten some label modifier bytes in case of error
        input.reset(&before_label_modifier);

        // if a label was present as well as :, we prefer to stop here
        if label.is_some() && followed_by_double_column.is_some() {
            input.reset(&before_double_column);
            return Ok((build_possible_label(), None));
        }

        // otherwise this is a normal stuff

        // we must have an instruction if label is missing; otherwise it is optional
        let instruction =
            opt(alt((parse_z80_directive_with_block, parse_single_token))).parse_next(input)?;

        if label.is_some() && instruction.is_none() {
            if let Ok(call) = parse_macro_or_struct_call_inner(false, label.take().unwrap()) // label is eaten
                .map(Some)
                .parse_next(input)
            {
                // this is a macro call
                let call = call.map(|t| t.into_located_token_between(&before_label, *input));
                my_space0.parse_next(input)?;

                Ok((None, call))
            }
            else {
                // this is a label
                Ok((build_possible_label(), None))
            }
        }
        else {
            // this cannot be a macro as there is an instruction
            my_space0.parse_next(input)?;
            Ok((build_possible_label(), instruction))
        }
    }
}

/// TODO - currently consume several lines. Should do it only one time
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_line_or_with_comment(
    input: &mut InnerZ80Span
) -> ModalResult<Option<LocatedToken>, Z80ParserError> {
    // let _ =opt(line_ending).parse_next(input)?;
    let _before_comment = *input;
    let comment = delimited(my_space0, opt(parse_comment), my_space0).parse_next(input)?;
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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_single_token(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
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

/// Accept "fname" as in most assemblers and fname as in vasm
pub fn parse_fname(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    alt((
        parse_string.map(|s: UnescapedString| LocatedExpr::String(s)),
        terminated(parse_label(false), not(alt(("/", "://"))))
            .map(|l| LocatedExpr::Label(l.into())),
        parse_stringlike_without_quote.map(|s: UnescapedString| LocatedExpr::String(s))
    ))
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_z80_directive_with_block(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let _ = my_space0(input)?;

    if input.state.options().is_orgams() {
        alt((
            parse_macro.context(StrContext::Label("Error in macro")),
            parse_repeat.context(StrContext::Label("Error in repetition")),
            parse_conditional.context(StrContext::Label("Error in condition")),
            parse_orgams_fail // TODO call it elsewhere
        ))
        .parse_next(input)
    }
    else {
        alt((
            parse_basic.context(StrContext::Label("Basic code embedding")),
            parse_macro.context(StrContext::Label("Error in macro")),
            parse_crunched_section.context(StrContext::Label("Error in crunched section")),
            parse_module.context(StrContext::Label("Error in module")),
            parse_confined.context(StrContext::Label("Error in confined")),
            parse_repeat.context(StrContext::Label("Error in repetition")),
            parse_for.context(StrContext::Label("Error in for")),
            parse_function.context(StrContext::Label("Error in function definition")),
            parse_switch.context(StrContext::Label("Error in switch")),
            parse_iterate.context(StrContext::Label("Error in iterate")),
            parse_while.context(StrContext::Label("Error in while")),
            parse_rorg.context(StrContext::Label("Error in rorg")),
            parse_conditional.context(StrContext::Label("Error in condition")),
            parse_assembler_control_max_passes_number
                .context(StrContext::Label("Error in assembler control"))
        ))
        .parse_next(input)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_lines(input: &mut InnerZ80Span) -> ModalResult<Vec<LocatedToken>, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_line(
    r#in: &mut Vec<LocatedToken>
) -> impl FnMut(&mut InnerZ80Span) -> ModalResult<(), Z80ParserError> + '_ {
    move |input: &mut InnerZ80Span| -> ModalResult<(), Z80ParserError> {
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
            input.reset(&before_end);
            None
        }
        else {
            let comment = opt(parse_comment).parse_next(input)?;

            alt((eof::<_, Z80ParserError>, line_ending))
                .value(())
                .context(StrContext::Label("Line ending expected"))
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
) -> impl FnMut(&mut InnerZ80Span) -> ModalResult<(), Z80ParserError> + '_ {
    parse_line(r#in)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assign_operator(
    input: &mut InnerZ80Span
) -> ModalResult<Option<BinaryOperation>, Z80ParserError> {
    let start = input.checkpoint();
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
            return Err(ErrMode::Cut(Z80ParserError::from_input(input).add_context(
                input,
                &start,
                "Wrong symbol"
            )));
        }
    };

    Ok(oper)
}

/// Parser for file names in appropriate directives
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_string(input: &mut InnerZ80Span) -> ModalResult<UnescapedString, Z80ParserError> {
    let opener = alt(('"', '\'')).parse_next(input)? as char;
    let closer = opener;
    let (normal, escapable) = match opener {
        '\'' => (none_of(('\\', '\'')).take(), one_of(('\\', '\''))),
        '"' => (none_of(('\\', '"')).take(), one_of(('\\', '"'))),
        _ => unreachable!()
    };

    let (string, slice) = terminated(
        opt(my_escaped(normal, '\\', escapable))
            .map(|s| s.unwrap_or_default())
            .with_taken(),
        closer.context(StrContext::Label("End of string not found"))
    )
    .parse_next(input)?;

    let slice = (*input).update_slice(slice);

    Ok(UnescapedString(string, slice.into()))
}

pub fn parse_stringlike_without_quote(
    input: &mut InnerZ80Span
) -> ModalResult<UnescapedString, Z80ParserError> {
    let (normal, escapable) = (
        none_of(('\\', ' ', '\r', '\n', ':', ';')),
        one_of(('\\', ' ', ':', ';'))
    );
    let (string, slice) = opt(my_escaped(normal, '\\', escapable))
        .map(|s| s.unwrap_or_default())
        .with_taken()
        .parse_next(input)?;

    let slice = (*input).update_slice(slice);

    Ok(UnescapedString(string, slice.into()))
}

#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn my_escaped<'a, I: 'a, Error, F, G, O1, O2>(
    mut normal: F,
    control_char: char,
    mut escapable: G
) -> impl Parser<I, String, Error>
where
    I: crate::parser::parser::winnow::stream::StreamIsPartial,
    I: Stream,
    <I as Stream>::Token: AsChar + Clone,
    <I as Stream>::Slice: AsBytes,
    I: cpclib_common::winnow::stream::Compare<char>,
    F: Parser<I, O1, Error>,
    G: Parser<I, O2, Error>,
    Error: ParserError<I> + Debug,
    O1: Debug,
    O2: Debug + AsChar
{
    move |input: &mut I| {
        let mut res = Vec::new();

        let start = input.checkpoint();

        while input.eof_offset() > 0 {
            let current_len = input.eof_offset();

            match opt(normal.by_ref().take()).parse_next(input)? {
                Some(c) => {
                    res.extend(c.as_bytes());
                    if input.eof_offset() == current_len {
                        return Ok(String::from_utf8_lossy(&res).into_owned());
                    }
                },
                None => {
                    if opt(control_char).parse_next(input)?.is_some() {
                        let c = escapable.parse_next(input)?;
                        let c = c.as_char();
                        let mut buffer = [0; 4];
                        let s = c.encode_utf8(&mut buffer);
                        res.extend(s.bytes());
                    }
                    else {
                        return Ok(String::from_utf8_lossy(&res).into_owned());
                    }
                },
            }
        }

        input.reset(&start);
        input.finish();
        Ok(String::from_utf8_lossy(&res).into_owned())
    }
}

pub fn parse_charset(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let charset =
        opt(alt((parse_charset_string, parse_charset_start_stop_end))).parse_next(input)?;

    Ok(charset
        .map(LocatedTokenInner::Charset)
        .unwrap_or_else(|| LocatedTokenInner::Charset(CharsetFormat::Reset)))
}

pub fn parse_charset_start_stop_end(
    input: &mut InnerZ80Span
) -> ModalResult<CharsetFormat, Z80ParserError> {
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

pub fn parse_charset_string(
    input: &mut InnerZ80Span
) -> ModalResult<CharsetFormat, Z80ParserError> {
    // manage the string format - TODO manage the others too
    let chars = parse_string
        .context(StrContext::Label("Missing string"))
        .parse_next(input)?;
    let chars = unsafe { std::str::from_utf8_unchecked(chars.as_ref().as_bytes()) };
    let start = preceded(parse_comma, expr)
        .context(StrContext::Label("Missing start value"))
        .parse_next(input)?;
    let format = CharsetFormat::CharsList(chars.chars().collect_vec(), start);

    Ok(format)
}

/// Parser for the include directive
pub fn parse_include(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let once_fname = (
        opt(delimited(my_space0, parse_word(b"ONCE"), my_space0)),
        cut_err(parse_fname.context(StrContext::Label("INCLUDE: error in fname")))
    )
        .parse_next(input)?;

    let (once, fname) = once_fname;

    let namespace = opt(preceded(
        delimited(
            my_space0,
            alt((Caseless("namespace"), Caseless("module"), Caseless("as"))),
            my_space0
        ),
        delimited(
            '"',
            parse_label(false),
            '"' // TODO modify to accept only labels without dot
        )
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Include(
        fname,
        namespace.map(|n| n.into()),
        once.is_some()
    ))
}

/// Parse for the various binary include directives
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_incbin(
    transformation: BinaryTransformation
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let fname = preceded(my_space0, parse_fname).parse_next(input)?;

        let offset =
            opt(preceded((my_space0, (','), my_space0), located_expr)).parse_next(input)?;
        let length =
            opt(preceded((my_space0, (','), my_space0), located_expr)).parse_next(input)?;
        let _extended_offset =
            opt(preceded((my_space0, (','), my_space0), expr)).parse_next(input)?;
        let off =
            opt(preceded((my_space0, (','), my_space0), Caseless("OFF"))).parse_next(input)?;

        Ok(LocatedTokenInner::Incbin {
            fname,
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
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // filter all the stuff before
    let _ = ((
        Caseless("DIRECT"),
        my_space1,
        Caseless("-1"),
        parse_comma,
        Caseless("-1"),
        parse_comma
    ))
        .parse_next(input)?;

    let bank =
        cut_err(located_expr.context(StrContext::Label("WRITE DIRECT -1, -1: BANK expected")))
            .parse_next(input)?;

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
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if save_kind == SaveKind::WriteDirect {
            (parse_word(b"DIRECT"), not((my_space0, "-1"))).parse_next(input)?;
        }
        else {
            not((parse_word(b"DIRECT"), my_space0, "-1")).parse_next(input)?;
        }

        let filename = located_expr.parse_next(input)?;

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
                    parse_word(b"ASCII").value(SaveType::Ascii),
                    parse_word(b"DSK").value(SaveType::Disc(DiscType::Dsk)),
                    parse_word(b"HFE").value(SaveType::Disc(DiscType::Hfe)),
                    parse_word(b"DISC").value(SaveType::Disc(DiscType::Auto)),
                    parse_word(b"TAPE").value(SaveType::Tape)
                ))
            ))
            .parse_next(input)?
        }
        else if save_kind == SaveKind::WriteDirect {
            Some(SaveType::AmsdosBin)
        }
        else {
            None
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
            filename,
            address: address.unwrap_or(None),
            size: size.unwrap_or(None),
            save_type,
            dsk_filename,
            side
        })
    }
}

/// Parse  UNDEF directive.
pub fn parse_undef(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let label = parse_label(false).parse_next(input)?;

    Ok(LocatedTokenInner::Undef(label.into()))
}

pub fn parse_section(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let name = preceded(my_space0, parse_label(false)).parse_next(input)?;

    Ok(LocatedTokenInner::Section(name.into()))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_range(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let start = cut_err(
        delimited(my_space0, located_expr, my_space0)
            .context(StrContext::Label("RANGE: wrong start address"))
    )
    .parse_next(input)?;
    let stop = cut_err(
        preceded(parse_comma, delimited(my_space0, located_expr, my_space0))
            .context(StrContext::Label("RANGE: wrong end address"))
    )
    .parse_next(input)?;
    let label = cut_err(
        preceded(
            parse_comma,
            delimited(my_space0, parse_label(false), my_space0)
        )
        .context(StrContext::Label("RANGE: wrong name"))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Range(label.into(), start, stop))
}
// pub fn parse_assign(input: &mut InnerZ80Span) -> ModalResult<TokenInner, Z80ParserError> {
// let ((label, op, value)) = ((
// parse_label(false),
// delimited(space0, parse_assign_operator, space0),
// expr
// )).parse_next(input)?;
//
// Ok((TokenInner::Assign{label, value, op}))
// }

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_token(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let parsing_state = input.state.state;

    alt((parse_token1, parse_token2))
        .verify(move |t| t.is_accepted(&parsing_state))
        .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_token1(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    parse_opcode_no_arg(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_token2(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let input_start = input.checkpoint();

    // Get the first word that will drive the rest of parsing
    let word = delimited(my_space0, alpha1, my_space0).parse_next(input)?;

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
        choice_nocase!(b"RLC") => alt((
            parse_shifts_and_rotations(Mnemonic::Rlc),
            parse_shifts_and_rotations_fake(Mnemonic::Rlc)
        )).parse_next(input),
        choice_nocase!(b"RL") => alt((
            parse_shifts_and_rotations(Mnemonic::Rl),
            parse_shifts_and_rotations_fake(Mnemonic::Rl)
        )).parse_next(input),
        choice_nocase!(b"RRC") => alt((
            parse_shifts_and_rotations(Mnemonic::Rrc),
            parse_shifts_and_rotations_fake(Mnemonic::Rrc)
        )).parse_next(input),
        choice_nocase!(b"RR") => alt((
            parse_shifts_and_rotations(Mnemonic::Rr),
            parse_shifts_and_rotations_fake(Mnemonic::Rr),
        )).parse_next(input),
        choice_nocase!(b"RST") => {
                alt((
                parse_rst_fake, parse_rst
            )).parse_next(input)
        },

        choice_nocase!(b"SBC") => parse_sbc.parse_next(input),
        choice_nocase!(b"SET") => parse_res_set_bit(Mnemonic::Set).parse_next(input),
        choice_nocase!(b"SL") /*1*/  => cut_err(preceded(('1', my_space1), parse_shifts_and_rotations(Mnemonic::Sl1))).parse_next(input),
        choice_nocase!(b"SLA") => alt((
            parse_shifts_and_rotations(Mnemonic::Sla),
            parse_shifts_and_rotations_fake(Mnemonic::Sla),
        )).parse_next(input),
        choice_nocase!(b"SLL") => alt((
            parse_shifts_and_rotations(Mnemonic::Sl1),
            parse_shifts_and_rotations_fake(Mnemonic::Sl1),
        )).parse_next(input),
        choice_nocase!(b"SRA") => alt((
            parse_shifts_and_rotations(Mnemonic::Sra),
            parse_shifts_and_rotations_fake(Mnemonic::Sra)
        )).parse_next(input),
        choice_nocase!(b"SRL") => alt((
            parse_shifts_and_rotations(Mnemonic::Srl),
            parse_shifts_and_rotations_fake(Mnemonic::Srl),
        )).parse_next(input),
        choice_nocase!(b"SUB") => parse_sub.parse_next(input),

        choice_nocase!(b"XOR") => parse_logical_operator(Mnemonic::Xor).parse_next(input),

        _ => {
            Err(ErrMode::Backtrack(Z80ParserError::from_input(
                input
            )))
        },
    }?;

    let token = token.into_located_token_between(&input_start, *input);
    Ok(token)
}

/// Parse ex af, af' instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ex_af(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ex_hl_de(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    alt((
        ((
            //          Caseless("EX"),
            //          space1,
            parse_register_hl,
            parse_comma,
            parse_register_de
        ))
            .value(()),
        ((
            //            Caseless("EX"),
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ex_mem_sp(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let destination = ((
        //     Caseless("EX"),
        //      space1,
        ('('),
        my_space0,
        parse_register_sp,
        my_space0,
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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_struct_directive(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    alt((
        parse_struct_directive_inner,
        parse_macro_or_struct_call(false, true)
    ))
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_struct_directive_inner(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    // XXX Sadly the state is stored within the context that cannot
    //     by changed. So we can cannot really use parsing state sutf

    let input_start = input.checkpoint();
    let parsing_state = ParsingState::StructLimited;
    let directive = parse_directive_new(&parsing_state.clone())
        .verify(move |d| d.is_accepted(&parsing_state))
        .parse_next(input)?;

    // Only one argument is allowed
    if (directive.is_db() || directive.is_dw()) && directive.data_exprs().len() > 1 {
        return Err(ErrMode::Cut(Z80ParserError::from_input(input).add_context(
            input,
            &input_start,
            "0 or 1 arguments are expected"
        )));
    }
    Ok(directive)
}

/// Parse any directive
pub fn parse_directive(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let parsing_state = input.state.state;
    parse_directive_new(&parsing_state.clone())
        .verify(move |d| d.is_accepted(&parsing_state))
        .parse_next(input)
}

/// Here local_parsing_state only serves to adapt DB/DW/STR behavior in struct.
/// Maybe it should be used to control the directives of interest BEFORE there parsing instead of after.
/// No filtering is done
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_directive_new(
    local_parsing_state: &ParsingState
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> + '_ {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedToken, Z80ParserError> {
        let is_orgams = input.state.options().is_orgams();

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

        let token: LocatedTokenInner = match word.len() {
            2 => parse_directive_of_size_2(input, &input_start, is_orgams, within_struct, word),
            3 => parse_directive_of_size3(input, &input_start, is_orgams, within_struct, word),
            4 => parse_directive_of_size_4(input, &input_start, is_orgams, within_struct, word),
            5 => parse_directive_of_size_5(input, &input_start, is_orgams, within_struct, word),
            6 => parse_directive_of_size_6(input, &input_start, is_orgams, within_struct, word),
            7 => parse_directive_of_size_7(input, &input_start, is_orgams, within_struct, word),
            8 => parse_directive_of_size_8(input, &input_start, is_orgams, within_struct, word),
            10 => parse_directive_of_size_10(input, &input_start, is_orgams, within_struct, word),
            _ => parse_directive_of_size_others(input, &input_start, is_orgams, within_struct, word)
        }?;

        let token = token.into_located_token_between(&input_start, *input);
        Ok(token)
    }
}

fn parse_directive_of_size_others(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match &word.to_ascii_uppercase()[..] {
        // 12
        #[cfg(not(target_arch = "wasm32"))]
        b"INCSHRINKLER" => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::Shrinkler)).parse_next(input)
        },

        // 13
        b"STARTINGINDEX" => parse_startingindex.parse_next(input),

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_10(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"ASMCONTROL") => parse_assembler_control.parse_next(input),
        choice_nocase!(b"BREAKPOINT") => parse_breakpoint.parse_next(input),
        choice_nocase!(b"DEFSECTION") => parse_range.parse_next(input),

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_8(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"BINCLUDE") => parse_incbin(BinaryTransformation::None).parse_next(input),
        choice_nocase!(b"BUILDSNA") => parse_buildsna(true).parse_next(input),
        choice_nocase!(b"BUILDCPR") => Ok(LocatedTokenInner::BuildCpr),
        choice_nocase!(b"INCLZSA1") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZSA1)).parse_next(input)
        },
        choice_nocase!(b"INCLZSA2") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZSA1)).parse_next(input)
        },
        choice_nocase!(b"NOEXPORT") => parse_export(ExportKind::NoExport).parse_next(input),
        choice_nocase!(b"WAITNOPS") => parse_waitnops.parse_next(input),
        choice_nocase!(b"SNAPINIT") => parse_snainit.parse_next(input),

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_7(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"INCLUDE") => parse_include.parse_next(input),
        choice_nocase!(b"BANKSET") => parse_bankset.parse_next(input),
        choice_nocase!(b"CHARSET") => parse_charset.parse_next(input),
        choice_nocase!(b"PROTECT") => parse_protect.parse_next(input),
        choice_nocase!(b"SECTION") => parse_section.parse_next(input),
        choice_nocase!(b"SNAINIT") => parse_snainit.parse_next(input),
        #[cfg(not(target_arch = "wasm32"))]
        choice_nocase!(b"INCUPKR") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::Upkr)).parse_next(input)
        },
        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_6(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"ASSERT") => parse_assert.parse_next(input),

        choice_nocase!(b"EXPORT") => parse_export(ExportKind::Export).parse_next(input),
        choice_nocase!(b"INCBIN") => parse_incbin(BinaryTransformation::None).parse_next(input),
        #[cfg(not(target_arch = "wasm32"))]
        choice_nocase!(b"INCEXO") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZEXO)).parse_next(input)
        },
        #[cfg(not(target_arch = "wasm32"))]
        choice_nocase!(b"INCLZ4") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ4)).parse_next(input)
        },

        choice_nocase!(b"INCL48") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ48)).parse_next(input)
        },

        choice_nocase!(b"INCL49") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZ49)).parse_next(input)
        },

        #[cfg(not(target_arch = "wasm32"))]
        choice_nocase!(b"INCAPU") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZAPU)).parse_next(input)
        },

        #[cfg(not(target_arch = "wasm32"))]
        choice_nocase!(b"INCZX0") => {
            parse_incbin(BinaryTransformation::Crunch(CrunchType::LZX0)).parse_next(input)
        },

        choice_nocase!(b"RETURN") => parse_return.parse_next(input),
        choice_nocase!(b"SNASET") => parse_snaset(true).parse_next(input),

        choice_nocase!(b"STRUCT") => parse_struct.parse_next(input),
        choice_nocase!(b"TICKER") => parse_stable_ticker.parse_next(input),

        choice_nocase!(b"NOLIST") => Ok(LocatedTokenInner::NoList),

        choice_nocase!(b"IMPORT") if is_orgams => parse_include.parse_next(input), /* TODO filter to remove the orgams specificies */

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_5(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"ALIGN") => parse_align.parse_next(input),
        choice_nocase!(b"ABYTE") => {
            parse_db_or_dw_or_str(DbDwStr::Abyte, within_struct).parse_next(input)
        },
        choice_nocase!(b"LIMIT") => parse_limit.parse_next(input),
        choice_nocase!(b"PAUSE") => Ok(LocatedTokenInner::Pause),
        choice_nocase!(b"PRINT") => parse_print(true).parse_next(input),
        choice_nocase!(b"RANGE") => parse_range.parse_next(input),
        choice_nocase!(b"UNDEF") => parse_undef.parse_next(input),

        choice_nocase!(b"WRITE") => {
            alt((parse_save(SaveKind::WriteDirect), parse_write_direct_memory)).parse_next(input)
        },

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_4(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"DEFB")
        | choice_nocase!(b"DEFM")
        | choice_nocase!(b"BYTE")
        | choice_nocase!(b"TEXT") => {
            parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)
        },

        choice_nocase!(b"FILL") | choice_nocase!(b"DEFS") | choice_nocase!(b"RMEM") => {
            parse_defs.parse_next(input)
        },

        choice_nocase!(b"BANK") => parse_bank.parse_next(input),
        choice_nocase!(b"FAIL") => parse_fail(true).parse_next(input),
        choice_nocase!(b"LIST") => Ok(LocatedTokenInner::List),
        choice_nocase!(b"READ") => parse_include.parse_next(input),

        choice_nocase!(b"SAVE") => parse_save(SaveKind::Save).parse_next(input),

        choice_nocase!(b"SKIP") if is_orgams => parse_skip.parse_next(input),

        choice_nocase!(b"WORD") | choice_nocase!(b"DEFW") => {
            parse_db_or_dw_or_str(DbDwStr::Dw, within_struct).parse_next(input)
        },
        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size3(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"BRK") if is_orgams => parse_breakpoint.parse_next(input),

        choice_nocase!(b"STR") => {
            parse_db_or_dw_or_str(DbDwStr::Str, within_struct).parse_next(input)
        },
        choice_nocase!(b"END") if !is_orgams => Ok(LocatedTokenInner::End),
        choice_nocase!(b"ENT") => parse_run(RunEnt::Ent).parse_next(input),
        choice_nocase!(b"MAP") => parse_map.parse_next(input),
        choice_nocase!(b"NOP") => parse_nop.parse_next(input),
        choice_nocase!(b"ORG") => parse_org.parse_next(input),
        choice_nocase!(b"RUN") => parse_run(RunEnt::Run).parse_next(input),
        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

fn parse_directive_of_size_2(
    input: &mut InnerZ80Span,
    input_start: &Checkpoint<
        Checkpoint<Checkpoint<&'static BStr, &'static BStr>, LocatingSlice<&'static BStr>>,
        Stateful<LocatingSlice<&'static BStr>, &'static context::ParserContext>
    >,
    is_orgams: bool,
    within_struct: bool,
    word: &[u8]
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    match word {
        choice_nocase!(b"BY") if is_orgams => {
            parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)
        },

        choice_nocase!(b"DB") | choice_nocase!(b"DM") => {
            parse_db_or_dw_or_str(DbDwStr::Db, within_struct).parse_next(input)
        },

        choice_nocase!(b"DS") => parse_defs.parse_next(input),

        choice_nocase!(b"DW") => {
            parse_db_or_dw_or_str(DbDwStr::Dw, within_struct).parse_next(input)
        },

        _ => {
            input.reset(input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_conditional(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let is_orgams = input.state.options().is_orgams();

    //  dbg!(&input);

    let if_clone = *input;
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

        //  dbg!(&if_token_or_error);

        // leave if the first loop does not have a test
        if first_loop && if_token_or_error.is_err() {
            input.reset(&if_start);
            return Err(if_token_or_error.err().unwrap());
        }

        // Get the current condition or nothing for the very last branch
        let condition = if let Ok(test_kind) = if_token_or_error {
            // Get the corresponding test
            let cond = cut_err(
                delimited(my_space0, parse_conditional_condition(test_kind), my_space0)
                    .context(StrContext::Label("Condition: error in the condition"))
            )
            .parse_next(input)?;
            Some(cond)
        }
        else {
            None
        };

        //   dbg!(&condition);

        // Remove empty stuff
        let _ = cut_err(
            alt((
                delimited(my_space0, parse_comment, line_ending).take(),
                line_ending.take(),
                ':'.take()
            ))
            .context(StrContext::Label(
                "Condition: condition must end by a new line or ':'"
            ))
        )
        .parse_next(input)
        .map_err(|e| e.add_context(input, &if_start, "Error in condition"))?;

        // get the conditionnal code
        // dbg!("Listing to extract code", &input);
        let code = cut_err(inner_code.context(StrContext::Label(
            "Condition: syntax error in conditionnal code"
        )))
        .parse_next(input)?;
        //  dbg!(unsafe{std::str::from_utf8_unchecked(input.as_bytes())});

        //  dbg!(&code);

        if let Some(condition) = condition {
            conditions.push((condition, code));

            let r#else = opt(preceded(
                repeat::<_, _, (), _, _>(0.., alt((
                    my_space1.value(()),
                    line_ending.value(()),
                    ':'.value(())
                ))),
                (winnow::ascii::Caseless(b"ELSE"), my_space0) // no word to allow ELSEIF in addition to ELSE IF
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
    //   dbg!("Everythng  has been read", &input);

    let _ = (
        opt(alt((
            delimited(my_space0, ':', my_space0).value(()),
            delimited(my_space0, parse_comment, line_ending).value(())
        ))),
        cut_err(preceded(
            my_space0,
            parse_directive_word(if is_orgams { b"END" } else { b"ENDIF" })
        ))
        .take()
    )
        .parse_next(input)
        .map_err(|e| e.add_context(&if_clone, &if_start, "End directive not found"))?;

    // dbg!(unsafe{std::str::from_utf8_unchecked(input.as_bytes())}); // endif must have been eaten

    let token = LocatedTokenInner::If(conditions, else_clause)
        .into_located_token_between(&if_start, *input);
    Ok(token)
}

/// Read the condition part in the parse_conditional macro
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_conditional_condition(
    code: KindOfConditional
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTestKind, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTestKind, Z80ParserError> {
        match &code {
            KindOfConditional::If => located_expr.map(LocatedTestKind::True).parse_next(input),

            KindOfConditional::IfNot => located_expr.map(LocatedTestKind::False).parse_next(input),

            KindOfConditional::IfDef => {
                preceded(my_space0, parse_label(false))
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

/// Parse a breakpoint instruction
pub fn parse_breakpoint(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let expr = opt(terminated(located_expr, not('='))
        .with_taken()
        .verify(|(e, s)| {
            // disallow labels that are similar to some keywords
            !s.eq_ignore_ascii_case(b"READ")
                && !s.eq_ignore_ascii_case(b"R")
                && !s.eq_ignore_ascii_case(b"WRITE")
                && !s.eq_ignore_ascii_case(b"W")
                && !s.eq_ignore_ascii_case(b"READWRITE")
                && !s.eq_ignore_ascii_case(b"RW")
                && !s.eq_ignore_ascii_case(b"MEM")
                && !s.eq_ignore_ascii_case(b"MEMORY")
                && !s.eq_ignore_ascii_case(b"EXEC")
                && !s.eq_ignore_ascii_case(b"EXECUTE")
                && !s.eq_ignore_ascii_case(b"STOP")
                && !s.eq_ignore_ascii_case(b"STOPPER")
                && !s.eq_ignore_ascii_case(b"WATCH")
                && !s.eq_ignore_ascii_case(b"WATCHER")
                && !s.contains(&b'=')
        })
        .map(|(e, s)| e))
    .parse_next(input)?;

    let address = Rc::new(RefCell::new(expr.map(|expr| (None, expr))));
    let r#type = Rc::new(RefCell::new(None));
    let access = Rc::new(RefCell::new(None));
    let run = Rc::new(RefCell::new(None));
    let mask = Rc::new(RefCell::new(None));
    let size = Rc::new(RefCell::new(None));
    let value = Rc::new(RefCell::new(None));
    let value_mask = Rc::new(RefCell::new(None));
    let condition = Rc::new(RefCell::new(None));
    let name = Rc::new(RefCell::new(None));
    let step = Rc::new(RefCell::new(None));

    let first = std::rc::Rc::new(std::cell::RefCell::new(true));

    loop {
        cut_err(
            opt(parse_breakpoint_argument)
                .verify_map(|arg| {
                    // at the same time verify if it is ok and update
                    if let Some(arg) = arg {
                        match arg {
                            BreakPointArgument::Address { arg, value } => {
                                let mut address = address.borrow_mut();
                                let address = address.deref_mut();
                                if address.is_some() {
                                    None
                                }
                                else {
                                    address.replace((Some(arg), value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Type { arg, value } => {
                                let mut r#type = r#type.borrow_mut();
                                let r#type = r#type.deref_mut();
                                if r#type.is_some() {
                                    None
                                }
                                else {
                                    r#type.replace((Some(arg), value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Access { arg, value } => {
                                let mut access = access.borrow_mut();
                                let access = access.deref_mut();
                                if access.is_some() {
                                    None
                                }
                                else {
                                    access.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Run { arg, value } => {
                                let mut run = run.borrow_mut();
                                let run = run.deref_mut();
                                if run.is_some() {
                                    None
                                }
                                else {
                                    run.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Mask { arg, value } => {
                                let mut item = mask.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Size { arg, value } => {
                                let mut item = size.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Value { arg, value: val } => {
                                let mut item = value.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, val));
                                    Some(())
                                }
                            },

                            BreakPointArgument::ValueMask { arg, value } => {
                                let mut item = value_mask.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Name { arg, value } => {
                                let mut item = name.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Condition { arg, value } => {
                                let mut item = condition.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            BreakPointArgument::Step { arg, value } => {
                                let mut item = step.borrow_mut();
                                let item = item.deref_mut();
                                if item.is_some() {
                                    None
                                }
                                else {
                                    item.replace((arg, value));
                                    Some(())
                                }
                            },

                            _ => Some(()) // TODO implement the tests
                        }
                    }
                    else if *first.borrow() {
                        Some(())
                    }
                    else {
                        None
                    }
                })
                .context(StrContext::Label("Breapoint parameter error"))
        )
        .parse_next(input)?;

        *first.borrow_mut() = false;

        if opt(parse_comma).parse_next(input)?.is_none() {
            break;
        }
    }

    let brk = LocatedTokenInner::Breakpoint {
        address: Rc::into_inner(address).unwrap().into_inner().map(|a| a.1),
        r#type: Rc::into_inner(r#type).unwrap().into_inner().map(|r| r.1),
        access: Rc::into_inner(access).unwrap().into_inner().map(|a| a.1),
        run: Rc::into_inner(run).unwrap().into_inner().map(|a| a.1),
        mask: Rc::into_inner(mask).unwrap().into_inner().map(|a| a.1),
        size: Rc::into_inner(size).unwrap().into_inner().map(|a| a.1),
        value: Rc::into_inner(value).unwrap().into_inner().map(|a| a.1),
        value_mask: Rc::into_inner(value_mask)
            .unwrap()
            .into_inner()
            .map(|a| a.1),
        condition: Rc::into_inner(condition).unwrap().into_inner().map(|a| a.1),
        name: Rc::into_inner(name).unwrap().into_inner().map(|a| a.1),
        step: Rc::into_inner(step).unwrap().into_inner().map(|a| a.1)
    };

    Ok(brk)
}

#[derive(Debug)]
pub enum BreakPointArgument {
    Type {
        arg: Option<InnerZ80Span>,
        value: RemuBreakPointType
    },
    Access {
        arg: Option<InnerZ80Span>,
        value: RemuBreakPointAccessMode
    },
    Run {
        arg: Option<InnerZ80Span>,
        value: RemuBreakPointRunMode
    },
    Address {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Mask {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Size {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Value {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    ValueMask {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Condition {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Name {
        arg: InnerZ80Span,
        value: LocatedExpr
    },
    Step {
        arg: InnerZ80Span,
        value: LocatedExpr
    }
}

pub fn parse_breakpoint_argument(
    input: &mut InnerZ80Span
) -> ModalResult<BreakPointArgument, Z80ParserError> {
    alt((
        parse_optional_argname_and_value("TYPE", &parse_breakpoint_type_value)
            .map(|(k, v)| BreakPointArgument::Type { arg: k, value: v }),
        parse_optional_argname_and_value("ACCESS", &parse_breakpoint_access_value)
            .map(|(k, v)| BreakPointArgument::Access { arg: k, value: v }),
        parse_optional_argname_and_value("RUNMODE", &parse_breakpoint_run_value)
            .map(|(k, v)| BreakPointArgument::Run { arg: k, value: v }),
        alt((
            parse_argname_and_value("ADDRESS", &located_expr),
            parse_argname_and_value("ADDR", &located_expr)
        ))
        .map(|(k, v)| BreakPointArgument::Address { arg: k, value: v }),
        parse_argname_and_value("MASK", &located_expr)
            .map(|(k, v)| BreakPointArgument::Mask { arg: k, value: v }),
        parse_argname_and_value("SIZE", &located_expr)
            .map(|(k, v)| BreakPointArgument::Size { arg: k, value: v }),
        parse_argname_and_value("VALUE", &located_expr)
            .map(|(k, v)| BreakPointArgument::Value { arg: k, value: v }),
        parse_argname_and_value("VALMASK", &located_expr)
            .map(|(k, v)| BreakPointArgument::ValueMask { arg: k, value: v }),
        parse_argname_and_value("STEP", &located_expr)
            .map(|(k, v)| BreakPointArgument::Step { arg: k, value: v }),
        parse_argname_and_value("CONDITION", &located_expr)
            .map(|(k, v)| BreakPointArgument::Condition { arg: k, value: v }),
        parse_argname_and_value("NAME", &located_expr)
            .map(|(k, v)| BreakPointArgument::Name { arg: k, value: v })
    ))
    .parse_next(input)
}

pub fn parse_breakpoint_type_value(
    input: &mut InnerZ80Span
) -> ModalResult<RemuBreakPointType, Z80ParserError> {
    parse_convertible_word(input)
}

pub fn parse_breakpoint_access_value(
    input: &mut InnerZ80Span
) -> ModalResult<RemuBreakPointAccessMode, Z80ParserError> {
    parse_convertible_word(input)
}

pub fn parse_breakpoint_run_value(
    input: &mut InnerZ80Span
) -> ModalResult<RemuBreakPointRunMode, Z80ParserError> {
    parse_convertible_word(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_convertible_word<T: FromStr>(
    input: &mut InnerZ80Span
) -> ModalResult<T, Z80ParserError> {
    delimited(my_space0, alpha1, my_space0)
        .verify_map(|word| T::from_str(unsafe { std::str::from_utf8_unchecked(word) }).ok())
        .parse_next(input)
}

pub fn parse_argname_to_assign(
    argname: &str
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> + use<'_> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let val = Caseless(argname).parse_next(input)?;
        let val = (*input).update_slice(val);

        (my_space0, '=', my_space0)
            .map(|(..)| val)
            .parse_next(input)
    }
}

pub fn parse_argname_and_value<'f, 's, O>(
    argname: &'s str,
    valparser: &'f dyn Fn(&mut InnerZ80Span) -> ModalResult<O, Z80ParserError>
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<(InnerZ80Span, O), Z80ParserError> + use<'f, 's, O> {
    move |input: &mut InnerZ80Span| (parse_argname_to_assign(argname), valparser).parse_next(input)
}

pub fn parse_optional_argname_and_value<'f, 's, O>(
    argname: &'s str,
    valparser: &'f dyn Fn(&mut InnerZ80Span) -> ModalResult<O, Z80ParserError>
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<(Option<InnerZ80Span>, O), Z80ParserError> + use<'f, 's, O>
{
    move |input: &mut InnerZ80Span| {
        alt((
            (
                parse_argname_to_assign(argname),
                cut_err(valparser.context(StrContext::Label("Wrong value for argument")))
            )
                .map(|(a, r)| (Some(a), r)),
            (valparser).map(|r| (None, r))
        ))
        .parse_next(input)
    }
}

pub fn parse_bankset(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let count = located_expr.parse_next(input)?;

    Ok(LocatedTokenInner::Bankset(count))
}

pub fn parse_buildsna(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"BUILDSNA").parse_next(input)?;
        }

        terminated(
            cut_err(opt(alt((
                Caseless("V2").value(SnapshotVersion::V2),
                Caseless("V3").value(SnapshotVersion::V3)
            ))))
            .map(|v: Option<SnapshotVersion>| LocatedTokenInner::BuildSna(v)),
            not(alphanumeric1)
        )
        .parse_next(input)
    }
}

#[derive(PartialEq)]
pub enum RunEnt {
    Run,
    Ent
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_run(kind: RunEnt) -> impl Parser<InnerZ80Span, LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let exp = cut_err(located_expr.context(match &kind {
            RunEnt::Run => "RUN expects at least one expression (e.g. RUN $)",
            RunEnt::Ent => "ENT expects one expression"
        }))
        .parse_next(input)?;

        let ga = if kind == RunEnt::Run {
            opt(preceded((my_space0, (','), my_space0), located_expr)).parse_next(input)?
        }
        else {
            None
        };

        Ok(LocatedTokenInner::Run(exp, ga))
    }
}

macro_rules! directive_with_expr {
    ($name:ident, $enum:tt) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        #[cfg_attr(target_arch = "wasm32", inline(never))]
        pub fn $name(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
            let exp = located_expr.parse_next(input)?;

            Ok((LocatedTokenInner::$enum(exp)))
        }
    };
}

directive_with_expr!(parse_map, Map);
directive_with_expr!(parse_limit, Limit);
directive_with_expr!(parse_waitnops, WaitNops);
directive_with_expr!(parse_return, Return);

pub fn parse_startingindex(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let start = opt(located_expr).parse_next(input)?;
    let step = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::StartingIndex { start, step })
}

pub fn parse_assembler_control(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    cut_err(
        alt((
            parse_assembler_control_print_parse,
            parse_assembler_control_print_any_pass
        ))
        .context(StrContext::Label(
            "Wrong argument in ASSEMBLING_CONTROL directive"
        ))
    )
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assembler_control_max_passes_number(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let asmctrl_start = input.checkpoint();

    let _ = preceded(
        my_space0,
        alt((parse_directive_word(b"ASMCONTROLENV"), my_space1))
    )
    .parse_next(input)?;

    let count = cut_err(preceded(
        (
            parse_word(b"SET_MAX_NB_OF_PASSES")
                .context(StrContext::Label("Missing modified option")),
            (my_space0, b'=', my_space0).context(StrContext::Label("Missing ="))
        ),
        located_expr.context(StrContext::Label("Expression expected"))
    ))
    .parse_next(input)?;

    let inner = cut_err(inner_code.context(StrContext::Label(
        "ASMCONTROLENV SET_MAX_NB_OF_PASSES: issue in the content"
    )))
    .parse_next(input)?;

    let _ = cut_err(
        preceded(
            my_space0,
            alt((
                parse_directive_word(b"ENDASMCONTROLENV"),
                parse_directive_word(b"ENDA")
            ))
        )
        .context(StrContext::Label("REPEAT: not closed"))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::AssemblerControl(
        LocatedAssemblerControlCommand::RestrictedAssemblingEnvironment {
            passes: Some(count),
            lst: inner
        }
    )
    .into_located_token_between(&asmctrl_start, *input))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assembler_control_print_any_pass(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    preceded(
        (parse_word(b"PRINT_ANY_PASS"), parse_comma),
        parse_print_inner
    )
    .map(|p| {
        LocatedTokenInner::AssemblerControl(LocatedAssemblerControlCommand::PrintAtAssemblingState(
            p
        ))
    })
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_assembler_control_print_parse(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let input2: InnerZ80Span = *input;

    preceded((parse_word(b"PRINT_PARSE"), parse_comma), parse_print_inner)
        .map(|p| {
            let msg = p.iter().map(|e| e.to_string()).join("");
            let ctx = input
                .state
                .current_filename
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_else(|| {
                    input
                        .state
                        .context_name()
                        .map(|c| c.to_owned())
                        .unwrap_or_default()
                });
            let (line, column) = Z80Span::from(input2).relative_line_and_column();
            println!("[PARSE] {ctx}:{line}:{column} {msg}");
            p
        })
        .map(|p| {
            LocatedTokenInner::AssemblerControl(
                LocatedAssemblerControlCommand::PrintAtParsingState(p)
            )
        })
        .parse_next(input)
}

/// Parse tickin directives
pub fn parse_stable_ticker(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    alt((parse_stable_ticker_start, parse_stable_ticker_stop)).parse_next(input)
}

/// Parse begining of ticker
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_stable_ticker_start(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    preceded(
        (Caseless("start"), alt((my_space1, parse_comma))),
        cut_err(parse_label(false).context(StrContext::Label("Missing label")))
    )
    .map(|name| LocatedTokenInner::StableTicker(StableTickerAction::<Z80Span>::Start(name.into())))
    .parse_next(input)
}

/// Parse end of ticker
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_stable_ticker_stop(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    Caseless("stop").parse_next(input)?;

    let name = opt(preceded(
        alt((my_space1, parse_comma)),
        parse_label(false).map(Z80Span::from)
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::StableTicker(
        StableTickerAction::<Z80Span>::Stop(name)
    ))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_bank(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let count = opt(located_expr).parse_next(input)?;

    Ok(LocatedTokenInner::Bank(count))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_skip(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let count = cut_err(located_expr.context(StrContext::Label("SKIP: wrong expression")))
        .parse_next(input)?;

    Ok(LocatedTokenInner::Skip(count))
}

/// Parse fake and real LD instructions
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ld(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        alt((
            parse_ld_fake(mnemonic_name_parsed),
            parse_ld_normal(mnemonic_name_parsed)
        ))
        .parse_next(input)
    }
}
/// Parse artifical LD instruction (would be replaced by several real instructions)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ld_fake(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !mnemonic_name_parsed {
            terminated(parse_word(b"LD"), my_space1).parse_next(input)?;
        }

        let dst = alt((
            terminated(
                alt((parse_register16, parse_indexregister16)),
                not(alt((Caseless(".low"), Caseless(".high"))))
            ),
            parse_hl_address,
            parse_indexregister_with_index
        ))
        .parse_next(input)?;

        let _ = parse_comma(input)?;

        // TODO - add https://z00m128.github.io/sjasmplus/documentation.html#s_fake_instructions

        let src = if dst.is_register_hl() {
            opt(parse_register_sp).parse_next(input)?
        }
        else {
            None
        };

        let src = if let Some(src) = src {
            src
        }
        else if dst.is_register16() {
            alt((
                terminated(
                    alt((parse_register16, parse_indexregister16)),
                    not(alt((Caseless(".low"), Caseless(".high"))))
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
                not(alt((Caseless(".low"), Caseless(".high"))))
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ld_normal(
    mnemonic_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !mnemonic_name_parsed {
            parse_word(b"LD").parse_next(input)?;
        }

        let _start = *input;
        let dst = cut_err(
            alt((
                parse_reg_address,
                parse_indexregister_address,
                parse_indexregister_with_index,
                parse_register_sp,
                terminated(
                    parse_register16,
                    not(alt((Caseless(".low"), Caseless(".high"))))
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

        let _ = cut_err(parse_comma.context(StrContext::Label("LD: missing comma")))
            .parse_next(input)?;

        // src possibilities depend on dst
        let src = cut_err(cut_err(parse_ld_normal_src(&dst)))
            .context(LD_WRONG_SOURCE)
            .parse_next(input)?;

        let token = LocatedTokenInner::new_opcode(Mnemonic::Ld, Some(dst), Some(src));

        Ok(token)
    }
}
/// Parse the source of LD depending on its destination
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_ld_normal_src(
    dst: &LocatedDataAccess
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> + '_ {
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
                        !((dst.is_register_h() || dst.is_register_l())
                            && (src.is_register_ixl()
                                || src.is_register_ixh()
                                || src.is_register_ixl()
                                || src.is_register_ixh()))
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
            input.reset(&input_start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
        }
    }
}

/// Parse RES, SET and BIT instructions
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_res_set_bit(
    res_or_set: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let bit = cut_err(parse_expr.context(StrContext::Label("Wrong bit definition")))
            .parse_next(input)?;

        let _ = cut_err(parse_comma).parse_next(input)?;

        let operand = cut_err(
            alt((
                parse_register8,
                parse_hl_address,
                parse_indexregister_with_index
            ))
            .context(StrContext::Label("Wrong destination"))
        )
        .parse_next(input)?;

        // Res can copy the result in a reg
        // not bit http://www.z80.info/z80undoc.htm
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_cp(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //   preceded(
    //    parse_word(b"CP"),

    preceded(
        opt((parse_register_a, parse_comma)),
        cut_err(
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
            .context(StrContext::Label("CP: wrong argument"))
        )
        .map(
            //   )
            |operand| LocatedTokenInner::new_opcode(Mnemonic::Cp, Some(operand), None)
        )
    )
    .parse_next(input)
}

#[derive(PartialEq)]
pub enum ExportKind {
    Export,
    NoExport
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_export(
    code: ExportKind
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let labels: Vec<InnerZ80Span> = cut_err(
            separated(0.., parse_label(false), parse_comma)
                .context(StrContext::Label("Wrong parameters"))
        )
        .parse_next(input)?;
        let labels = labels.into_iter().map(Z80Span::from).collect_vec();

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
    Abyte,
    Db,
    Dw,
    Str
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
/// Parse DB DW directives
pub fn parse_db_or_dw_or_str(
    code: DbDwStr,
    empty_list_allowed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let abyte_delta = if code == DbDwStr::Abyte {
            Some(
                cut_err(
                    terminated(located_expr, parse_comma)
                        .context(StrContext::Label("ABYTE: delta issue"))
                )
                .parse_next(input)?
            )
        }
        else {
            None
        };

        // STRUCT directive allows to have no arguments
        let expr = if empty_list_allowed {
            expr_list.parse_next(input).unwrap_or(Default::default())
        }
        else {
            expr_list
                .context(match code {
                    DbDwStr::Abyte => "ABYTE: error in arguments",
                    DbDwStr::Dw => "DEFW: error in arguments",
                    DbDwStr::Db => "DEFB: error in arguments",
                    DbDwStr::Str => "STR: error in arguments"
                })
                .parse_next(input)?
        };

        Ok(match code {
            DbDwStr::Db => LocatedTokenInner::Defb(expr),
            DbDwStr::Dw => LocatedTokenInner::Defw(expr),
            DbDwStr::Str => LocatedTokenInner::Str(expr),
            DbDwStr::Abyte => LocatedTokenInner::Abyte(abyte_delta.unwrap(), expr)
        })
    }
}

// Fail if we do not read a forbidden keyword
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_forbidden_keyword(
    input: &mut InnerZ80Span
) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let start = input.checkpoint();
    let _ = my_space0(input)?;
    let name = take_while(1.., ('a'..='z', 'A'..='Z', '0'..='9', '_'..='_'))
        .context(StrContext::Label("Unable to read directive name"))
        .parse_next(input)?;

    let mut end_directive_iter = if input.state.options().dotted_directive {
        DOTTED_END_DIRECTIVE.iter()
    }
    else {
        END_DIRECTIVE.iter()
    };

    let name = (*input).update_slice(name);

    if !end_directive_iter.any(|&a| a == name.to_ascii_uppercase()) {
        input.reset(&start);
        return Err(ErrMode::Backtrack(Z80ParserError::from_input(&name)));
    }

    let _ = my_space0(input)?;

    Ok(name)
}
pub fn parse_macro_arg(input: &mut InnerZ80Span) -> ModalResult<LocatedMacroParam, Z80ParserError> {
    let _start_input = input.checkpoint();
    let cloned = *input;

    let param = alt((
        delimited(
            (my_space0, ('[')),
            separated(0.., parse_macro_arg, ','),
            ((']'), my_space0)
        )
        .map(|l: Vec<LocatedMacroParam>| {
            LocatedMacroParam::List(
                l.into_iter()
                    .map(|p| Box::new(p.clone()))
                    .collect::<Vec<_>>()
            )
        }),
        delimited(
            my_space0,
            (
                opt(Caseless("{eval}").value(())),
                alt((
                    located_expr.take(), // TODO handle evaluation or transposition
                    parse_string.take(),
                    repeat::<_, _, (), _, _>(
                        0..,
                        none_of((b' ', b',', b'\r', b'\n', b'\t', b']', b'[', b';', b':'))
                    )
                    .take()
                ))
            ), // TODO find a way to give arguments with space
            alt((my_space0.value(()), eof.value(())))
        )
        .map(|(eval, s)| (eval.is_some(), cloned.update_slice(s)))
        .map(|(eval, arg)| (eval, Z80Span::from(arg)))
        .map(|(eval, arg)| {
            if eval {
                LocatedMacroParam::EvaluatedArgument(arg)
            }
            else {
                LocatedMacroParam::RawArgument(arg)
            }
        })
    ))
    .parse_next(input)?;

    Ok(param)
}

/// Manage the call of a macro.
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_macro_or_struct_call_inner(
    for_struct: bool,
    name: InnerZ80Span
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| {
        let input_start = input.checkpoint();

        my_space0.parse_next(input)?;
        not(':').parse_next(input)?;

        if !ignore_ascii_case_allowed_label(
            name.as_bstr(),
            input.state.options().dotted_directive,
            input.state.options().assembler_flavor
        ) {
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_input(input).add_context(
                    input,
                    &input_start,
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
        // alt((parse_comment.take(), ':', '\n'))
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

        let has_parenthesis = opt((
            '(',
            my_space0,
            not(alt((("void", my_space0).value(()), ')'.value(()))))
        ))
        .parse_next(input)?
        .is_some();
        let args: Vec<(LocatedMacroParam, &[u8])> = if peek(alt((
            eof::<_, Z80ParserError>.value(()),
            parse_comment.value(()),
            '\n'.value(()),
            ':'.value(())
        )))
        .parse_next(input)
        .is_ok()
        {
            vec![]
        }
        else {
            cut_err(
                alt((
                    delimited(
                        my_space0,
                        alt((
                            "()".value(()),
                            Caseless("(void)").value(()),
                            parse_comment.value(())
                        )),
                        my_space0
                    )
                    .value(Default::default()),
                    alt((
                        alt((Caseless("(void)"), "()")).value(Vec::new()),
                        separated(
                            1..,
                            alt((
                                parse_macro_arg.with_taken(),
                                my_space1
                                    .map(|space: InnerZ80Span| {
                                        LocatedMacroParam::RawArgument(space.into())
                                        // string of size 0;
                                    })
                                    .with_taken()
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

        if has_parenthesis {
            (my_space0, ')', my_space0).parse_next(input)?;
        }

        if args.len() == 1 && args.first().unwrap().0.is_empty() {
            panic!();
        }

        // avoid ambiguate code such as label nop
        if args.len() == 1 {
            let mut arg = (*input).update_slice(args[0].1);
            if alt((parse_word(b"NOP").take(), parse_opcode_no_arg.take()))
                .parse_next(&mut arg)
                .is_ok()
            {
                return Err(ErrMode::Cut(Z80ParserError::from_input(input).add_context(
                    input,
                    &input_start,
                    if for_struct {
                        "First argument of STRUCT cannot be an opcode with no argument"
                    }
                    else {
                        "First argument of MACRO or STRUCT cannot be an opcode with no argument"
                    }
                )));
            }
        }

        let args = args.into_iter().map(|(a, _b)| a).collect_vec();
        Ok(LocatedTokenInner::MacroCall(name.into(), args))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
/// TODO remove by restore the way to parse the macro name
pub fn parse_macro_or_struct_call(
    allowed_to_return_a_label: bool,
    for_struct: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedToken, Z80ParserError> {
        my_space0(input)?;
        let input_start = input.checkpoint();
        let name = terminated(
            parse_macro_name,
            not(alt((
                (
                    my_space0,
                    alt((':'.value(()), line_ending.value(()), eof.value(())))
                )
                    .take(),
                ('.').take()
            )))
        )
        .parse_next(input)?;

        // Check if the macro name is allowed
        if !ignore_ascii_case_allowed_label(
            name.as_bstr(),
            input.state.options().dotted_directive,
            input.state.options().assembler_flavor
        ) {
            return Err(ErrMode::Backtrack(
                Z80ParserError::from_input(input).add_context(
                    input,
                    &input_start,
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
        let inner = inner.into_located_token_between(&input_start, *input);
        Ok(inner)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_directive_word(
    name: &'static [u8]
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> + 'static {
    move |input: &mut InnerZ80Span| {
        if input.state.options().dotted_directive {
            preceded(b'.', parse_word(name)).parse_next(input)
        }
        else {
            parse_word(name).parse_next(input)
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
/// Consume the word and the empty space after
fn parse_word(
    name: &'static [u8]
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<InnerZ80Span, Z80ParserError> {
        let word = terminated(
            Caseless(name),
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

        let word = (*input).update_slice(word);
        Ok(word)
    }
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_djnz(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    preceded(opt(parse_comma), parse_expr)
        .map(|expr| LocatedTokenInner::new_opcode(Mnemonic::Djnz, Some(expr), None))
        .parse_next(input)
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn expr_list(input: &mut InnerZ80Span) -> ModalResult<Vec<LocatedExpr>, Z80ParserError> {
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
pub fn parse_assert(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let expr = cut_err(located_expr.context(StrContext::Label("ASSERT: expression error")))
        .parse_next(input)?;

    let exps = cut_err(
        opt(preceded(parse_comma, parse_print_inner))
            .context(StrContext::Label("ASSERT: comment error"))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Assert(expr, exps))
}

/// ...
pub fn parse_align(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let boundary = located_expr.parse_next(input)?;
    let fill = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::Align(boundary, fill))
}

pub fn parse_print_inner(
    input: &mut InnerZ80Span
) -> ModalResult<Vec<FormattedExpr>, Z80ParserError> {
    separated(
        1..,
        alt((
            formatted_expr,
            expr.map(FormattedExpr::from),
            parse_string.map({
                |s: UnescapedString| {
                    let s = s.as_ref();
                    FormattedExpr::from(Expr::String(SmolStr::from_iter(s.chars())))
                }
            })
        )),
        parse_comma
    )
    .parse_next(input)
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_print(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"PRINT").parse_next(input)?;
        }

        cut_err(parse_print_inner)
            .map(LocatedTokenInner::Print)
            .parse_next(input)
    }
}

pub fn parse_fail(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"FAIL").parse_next(input)?;
        }

        opt(parse_print_inner)
            .map(LocatedTokenInner::Fail)
            .parse_next(input)
    }
}

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn formatted_expr(input: &mut InnerZ80Span) -> ModalResult<FormattedExpr, Z80ParserError> {
    let _ = ('{').parse_next(input)?;
    let format = alt((
        Caseless("INT").value(ExprFormat::Int),
        Caseless("HEX4").value(ExprFormat::Hex(Some(4))),
        Caseless("HEX8").value(ExprFormat::Hex(Some(8))),
        Caseless("HEX2").value(ExprFormat::Hex(Some(2))),
        Caseless("HEX").value(ExprFormat::Hex(None)),
        Caseless("BIN8").value(ExprFormat::Bin(Some(8))),
        Caseless("BIN16").value(ExprFormat::Bin(Some(16))),
        Caseless("BIN32").value(ExprFormat::Bin(Some(32))),
        Caseless("BIN").value(ExprFormat::Bin(None))
    ))
    .parse_next(input)?;
    let _ = ('}').parse_next(input)?;

    let _ = my_space0(input)?;

    let exp = expr(input)?;

    Ok(FormattedExpr::Formatted(format, exp))
}

/// Handle \ in end of line
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn my_space0(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    opt(my_space1)
        .take()
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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn my_repeat1_<I, O, C, E, F>(f: &mut F, i: &mut I) -> ModalResult<C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>
{
    let start = i.checkpoint();
    match f.parse_next(i) {
        #[allow(deprecated)]
        Err(e) => Err(e.append(i, &start, ErrorKind::Many)),
        Ok(o) => {
            let mut acc = C::initial(None);
            acc.accumulate(o);

            loop {
                let start = i.checkpoint();
                let len = i.eof_offset();
                match f.parse_next(i) {
                    Err(ErrMode::Backtrack(_)) => {
                        i.reset(&start);
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn my_space1(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;

    let spaces = alt((
        eof.value(()).context(StrContext::Label("End of file")), // end of file
        one_of(|c: u8| c.is_space())
            .value(())
            .context(StrContext::Label("Space")), // space char
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
            .context(StrContext::Label("continuated line")),
        (space0, '\\', space0)
            .value(())
            .context(StrContext::Label("new line request")),
        parse_multiline_comment.value(())
    ));

    my_repeat1::<_, _, (), Z80ParserError, _>(spaces)
        .take()
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn my_line_ending(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    alt((line_ending.take(), ':'.take()))
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_comma(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    delimited(my_space0, ','.take(), my_space0)
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

fn parse_comma_multiline(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    (parse_comma, opt((newline, my_space0)))
        .take()
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

/// ...
pub fn parse_protect(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let start = located_expr.parse_next(input)?;

    let end = preceded(parse_comma, located_expr).parse_next(input)?;

    Ok(LocatedTokenInner::Protect(start, end))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
/// ...
pub fn parse_logical_operator(
    operator: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        // we optionaly allow a, as a first register
        let operand = preceded(
            opt((parse_register_a, my_space0, parse_comma, my_space0)),
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
        )
        .context(StrContext::Label("Wrong logical operand"))
        .parse_next(input)?;

        Ok(LocatedTokenInner::new_opcode(operator, Some(operand), None))
    }
}

/// Substraction with A register
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_sub(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =Caseless("SUB").parse_next(input)?;
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_sbc(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =Caseless("SBC").parse_next(input)?;
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_add_or_adc(
    add_or_adc: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
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
            return Err(ErrMode::Cut(Z80ParserError::from_input(input)));
        }?;

        Ok(LocatedTokenInner::new_opcode(
            add_or_adc,
            first,
            Some(second)
        ))
    }
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_push_n_pop(
    push_or_pop: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let mut registers: Vec<_> = separated(
            1..,
            alt((parse_register16, parse_indexregister16)),
            parse_comma
        )
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_ret(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let (cond, cond_bytes) = opt(parse_flag_test).with_taken().parse_next(input)?;

    let token = LocatedTokenInner::new_opcode(
        Mnemonic::Ret,
        cond.map(|cond| {
            LocatedDataAccess::FlagTest(cond, (*input).update_slice(cond_bytes).into())
        }),
        None
    );

    Ok(token)
}

/// ...
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_inc_dec(
    inc_or_dec: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_out(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    //  let _ =parse_word(b"OUT").parse_next(input)?;

    // get the port proposal
    let port = alt((parse_portc, parse_portnn)).parse_next(input)?;

    // the vlaue depends on the port
    let cloned = *input;
    let (value, span) = if port.is_port_c() {
        // reg c
        opt(preceded(
            parse_comma,
            alt((
                parse_register8,
                alt((parse_word(b"f").take(), "0")).map(|w| {
                    LocatedDataAccess::Expression(LocatedExpr::Value(
                        0,
                        cloned.update_slice(w).into()
                    ))
                })
            ))
        ))
        .with_taken()
        .parse_next(input)?
    }
    else {
        preceded(parse_comma, parse_register_a)
            .map(Some)
            .with_taken()
            .parse_next(input)?
    };

    let cloned = *input;
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_in(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"IN").parse_next(input)?;
    let cloned = *input;
    // get the port proposal
    let (destination, span) = opt(terminated(
        alt((
            parse_register8,
            alt((Caseless("f").take(), "0")).map(|span| {
                LocatedDataAccess::Expression(LocatedExpr::Value(
                    0,
                    cloned.update_slice(span).into()
                ))
            })
        )),
        parse_comma
    ))
    .with_taken()
    .parse_next(input)?;

    let cloned = *input;
    let destination = destination.unwrap_or(LocatedDataAccess::Expression(LocatedExpr::Value(
        0,
        cloned.update_slice(span).into()
    )));

    let port = cut_err(alt((
        parse_portc,
        parse_portnn.verify(|_| {
            destination
                .get_register8()
                .map(|r| r.is_a())
                .unwrap_or(false)
        })
    )))
    .parse_next(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::In,
        Some(destination),
        Some(port)
    ))
}

/// Parse the rst instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_rst(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    // let _ =parse_word(b"RST").parse_next(input)?;
    let val = parse_expr(input)?;

    Ok(LocatedTokenInner::new_opcode(
        Mnemonic::Rst,
        Some(val),
        None
    ))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_rst_fake(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let (flag, _, val) = (
        parse_flag_test
            .verify(|t| {
                t == &FlagTest::Z || t == &FlagTest::NZ || t == &FlagTest::C || t == &FlagTest::NC
            })
            .with_taken(),
        parse_comma,
        parse_expr
    )
        .parse_next(input)?;

    let flag = {
        let span = (*input).update_slice(flag.1);
        LocatedDataAccess::FlagTest(flag.0, span.into())
    };

    let token = LocatedTokenInner::new_opcode(Mnemonic::Rst, Some(flag), Some(val));
    let warning = LocatedTokenInner::WarningWrapper(
        Box::new(token),
        "This is a fake instruction assembled using several opcodes".into()
    );

    Ok(warning)
}

/// Parse the IM instruction
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_im(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_shifts_and_rotations(
    oper: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let _start = *input;
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

pub fn parse_shifts_and_rotations_fake(
    oper: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let _start = *input;
        let arg = alt((parse_register16,)).parse_next(input)?;

        let token = LocatedTokenInner::new_opcode(oper, Some(arg), None);
        let warning = LocatedTokenInner::WarningWrapper(
            Box::new(token),
            "This is a fake instruction assembled using several opcodes".into()
        );

        Ok(warning)
    }
}

/// TODO reduce the flag space for jr"],
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_call_jp_or_jr(
    call_jp_or_jr: Mnemonic
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        let _start = *input;

        let flag_test =
            opt(terminated(parse_flag_test.with_taken(), parse_comma)).parse_next(input)?;

        let dst = cut_err(
            alt((

                    alt((
                        parse_hl_address,
                        parse_indexregister_address,
                        parse_register_hl,
                        parse_indexregister16
                    ))
                .verify(|_| call_jp_or_jr.is_jp() && flag_test.is_none()), // not possible for call and for jp/jr when there is flag
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

        let flag_test = flag_test.map(|(f, s)| {
            let span = (*input).update_slice(s);
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_flag_test(input: &mut InnerZ80Span) -> ModalResult<FlagTest, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_register16(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let _start = input.checkpoint();
    let code = terminated(take(2usize), not(alpha1)).parse_next(input)?;

    let reg = match code {
        choice_nocase!(b"AF") => Register16::Af,
        choice_nocase!(b"BC") => Register16::Bc,
        choice_nocase!(b"DE") => Register16::De,
        choice_nocase!(b"HL") => Register16::Hl,
        _ => return Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
    };

    let span = (*input).update_slice(code);
    let reg = LocatedDataAccess::Register16(reg, span.into());

    Ok(reg)
}

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_register8(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    #[derive(PartialEq)]
    enum Reg16Modifier {
        Low,
        High
    };

    alt((
        ((
            parse_register16,
            preceded(
                b'.',
                alt((
                    Caseless("low").map(|_| Reg16Modifier::Low),
                    Caseless("high").map(|_| Reg16Modifier::High)
                ))
            ),
            my_space0
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_register_i(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let da = ((Caseless("I"), not(alphanumeric1)))
        .take()
        .parse_next(input)?;
    let da = LocatedDataAccess::SpecialRegisterI((*input).update_slice(da).into());
    Ok(da)
}

/// Parse register r
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_register_r(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let da = ((Caseless("R"), not(alphanumeric1)))
        .take()
        .parse_next(input)?;
    let da = LocatedDataAccess::SpecialRegisterR((*input).update_slice(da).into());
    Ok(da)
}

macro_rules! parse_any_register8 {
    ($name:ident, $char:expr, $reg:expr) => {
        /// Parse register $char
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        #[cfg_attr(target_arch = "wasm32", inline(never))]
        pub fn $name(i: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn register16_parser(
    representation: &'static str,
    register: Register16
) -> impl for<'src, 'ctx> Fn(&mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let span = ((
            Caseless(representation),
            not(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_')))
        ))
            .take()
            .parse_next(input)?;

        let span = (*input).update_slice(span);

        Ok(LocatedDataAccess::Register16(register, span.into()))
    }
}

macro_rules! parse_any_register16 {
    ($name:ident, $char:expr, $reg:expr) => {
        /// Parse the $char register and return it as a DataAccess
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        #[cfg_attr(target_arch = "wasm32", inline(never))]
        pub fn $name(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_register_ix(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    parse_indexregister16
        .verify(|d: &LocatedDataAccess| d.is_register_ix())
        .parse_next(input)
}

/// Parse the IY register
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_register_iy(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    parse_indexregister16
        .verify(|d: &LocatedDataAccess| d.is_register_iy())
        .parse_next(input)
}

// TODO find a way to not use that
macro_rules! parse_any_indexregister8 {
    ($($reg:ident, $alias1:ident, $alias2:ident)*) => {$(
        paste::paste! {
            /// Parse register $reg
            #[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
            pub fn [<parse_register_ $reg:lower>] (input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
                let _start = input.clone();
                let span = ((
                    alt((
                        parse_word( stringify!($reg).as_bytes()),
                        parse_word( stringify!($alias1).as_bytes()),
                        parse_word( stringify!($alias2).as_bytes()),
                    ))
                    , not(alphanumeric1)))
                .take()
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_indexregister8(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    alt((
        parse_register_ixh,
        parse_register_iyh,
        parse_register_ixl,
        parse_register_iyl
    ))
    .parse_next(input)
}

/// Parse a 16 bits indexed register
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_indexregister16(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let code = terminated(take(2usize), not(alpha1))
        .take()
        .parse_next(input)?;

    let reg = match code {
        choice_nocase!(b"IX") => IndexRegister16::Ix,
        choice_nocase!(b"IY") => IndexRegister16::Iy,
        _ => return Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
    };

    let span = (*input).update_slice(code);
    let reg = LocatedDataAccess::IndexRegister16(reg, span.into());

    Ok(reg)
}

/// Parse the use of an indexed register as (IX + 5)"
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_indexregister_with_index(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let start_checkpoint = input.checkpoint();
    let start_eof_offset = input.eof_offset();
    let (open, _, reg) =
        ((alt((b'(', b'[')), my_space0, parse_indexregister16)).parse_next(input)?;

    let op = opt(preceded(
        my_space0,
        alt((
            b'+'.value(BinaryOperation::Add),
            b'-'.value(BinaryOperation::Sub)
        ))
    ))
    .parse_next(input)?;

    let close = if open == b'(' { b')' } else { b']' };

    let expr = if op.is_some() {
        terminated(located_expr, (my_space0, close)).parse_next(input)?
    }
    else {
        (my_space0, close)
            .value(LocatedExpr::Value(0, (*input).into()))
            .parse_next(input)?
    };

    let span = build_span(start_eof_offset, &start_checkpoint, *input);
    Ok(LocatedDataAccess::IndexRegister16WithIndex(
        reg.get_indexregister16().unwrap(),
        op.unwrap_or(BinaryOperation::Add),
        expr,
        span.into()
    ))
}

/// Parse (C) used in in/out
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_portc(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let span = alt((
        ((b'(', my_space0, parse_register_c, my_space0, b')')),
        ((b'[', my_space0, parse_register_c, my_space0, b']'))
    ))
    .take()
    .parse_next(input)?;
    let span = (*input).update_slice(span);

    Ok(LocatedDataAccess::PortC(span.into()))
}

/// Parse (nn) used in in/out
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_portnn(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let (address, span) = alt((
        delimited("(", located_expr, preceded(my_space0, ")")),
        delimited("[", located_expr, preceded(my_space0, "]"))
    ))
    .with_taken()
    .parse_next(input)?;
    let span = (*input).update_slice(span);

    Ok(LocatedDataAccess::PortN(address, span.into()))
}

/// Parse an address access `(expression)`
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_address(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    // let filter = |c: u8| {
    // c == b'/'
    // || c == b'+'
    // || c == b'='
    // || c == b'-'
    // || c == b'*'
    // || c == b'<'
    // || c == b'>'
    // || c == b'%'
    // || c == b'&'
    // || c == b'|'
    // };
    let first_char = alt((b'(', b'[')).parse_next(input)?;
    let address = terminated(
        located_expr,
        (
            my_space0,
            if first_char == b'(' { b')' } else { b']' },
            peek(
                // filter expressions ; they are followed by some operators
                preceded(
                    my_space0,
                    alt((
                        eof.value(()),
                        my_line_ending.value(()),
                        ','.value(()),
                        ':'.value(()),
                        ';'.value(()),
                        "//".value(())
                    ))
                )
            )
        )
    )
    .parse_next(input)?;

    Ok(LocatedDataAccess::Memory(address))
}

/// Parse (R16)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_reg_address(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let (reg, span) = alt((
        delimited(
            terminated("(", my_space0),
            parse_register16,
            preceded(my_space0, ")")
        ),
        delimited(
            terminated("[", my_space0),
            parse_register16,
            preceded(my_space0, "]")
        )
    ))
    .with_taken()
    .parse_next(input)?;

    let da = LocatedDataAccess::MemoryRegister16(
        reg.get_register16().unwrap(),
        (*input).update_slice(span).into()
    );
    Ok(da)
}

/// Parse (HL)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_hl_address(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let span = alt((
        delimited(
            terminated("(", my_space0),
            parse_register_hl,
            preceded(my_space0, ")")
        ),
        delimited(
            terminated("[", my_space0),
            parse_register_hl,
            preceded(my_space0, "]")
        )
    ))
    .take()
    .parse_next(input)?;

    Ok(LocatedDataAccess::MemoryRegister16(
        Register16::Hl,
        (*input).update_slice(span).into()
    ))
}

/// Parse (ix) and (iy)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_indexregister_address(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let (reg, res) = delimited(
        terminated("(", my_space0),
        parse_indexregister16,
        preceded(my_space0, ")")
    )
    .with_taken()
    .parse_next(input)?;

    let span = (*input).update_slice(res);
    Ok(LocatedDataAccess::MemoryIndexRegister16(
        reg.get_indexregister16().unwrap(),
        span.into()
    ))
}

/// Parse an expression and returns it inside a DataAccession::Expression
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_expr(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let expr = located_expr.parse_next(input)?;
    Ok(LocatedDataAccess::Expression(expr))
}

/// Parse standard org directive
pub fn parse_org(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let val1 =
        cut_err(located_expr.context(StrContext::Label("Invalid argument"))).parse_next(input)?;
    let val2 = opt(preceded(parse_comma, located_expr)).parse_next(input)?;

    Ok(LocatedTokenInner::Org { val1, val2 })
}

/// Parse defs instruction. TODO add optional parameters
pub fn parse_defs(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let val = separated(
        1..,
        cut_err(
            ((located_expr, opt(preceded(parse_comma, located_expr))))
                .context(StrContext::Label("Wrong argument"))
        ),
        parse_comma
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::Defs(val))
}

pub fn parse_nop(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let val = cut_err(
        opt(located_expr.map(LocatedDataAccess::from)).context(StrContext::Label(
            "Wrong argument. NOP expects an expression"
        ))
    )
    .parse_next(input)?;

    Ok(LocatedTokenInner::OpCode(Mnemonic::Nop, val, None, None))
}

/// Parse any opcode having no argument
pub fn parse_opcode_no_arg(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let cloned = *input;
    let input_start = input.checkpoint();

    let token: LocatedToken = preceded(
        my_space0,
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
    .with_taken()
    .map(|(mne, span)| {
        let span = cloned.update_slice(span);
        LocatedTokenInner::OpCode(mne, None, None, None).into_located_token_at(span)
    })
    .parse_next(input)?;

    // http://rasm.wikidot.com/directives:repete
    // Some instructions may have repeated counts, so we modify them
    let token: LocatedToken = match &token.inner.as_ref().left().unwrap() {
        LocatedTokenInner::OpCode(
            Mnemonic::Ldi
            | Mnemonic::Ldd
            | Mnemonic::Rlca
            | Mnemonic::Rrca
            | Mnemonic::Ini
            | Mnemonic::Ind
            | Mnemonic::Outi
            | Mnemonic::Outd
            | Mnemonic::Halt,
            located_data_access,
            located_data_access1,
            register8
        ) => {
            debug_assert!(located_data_access.is_none());
            debug_assert!(located_data_access1.is_none());
            debug_assert!(register8.is_none());

            let repeat = opt(preceded(my_space1, located_expr)).parse_next(input)?;
            if let Some(repeat) = repeat {
                LocatedTokenInner::RepeatToken {
                    token: Box::new(token),
                    repeat
                }
                .into_located_token_between(&input_start, *input)
            }
            else {
                token
            }
        },

        _ => token
    };

    Ok(token)
}

fn parse_snainit(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let fname = parse_fname(input)?;

    Ok(LocatedTokenInner::SnaInit(fname))
}

fn parse_struct(input: &mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    let name = cut_err(parse_label(false)).parse_next(input)?;

    // TODO parse inner with filtering on the allowed operations
    // would be easier to write and would allow conditional operations
    let fields: Vec<(Z80Span, LocatedToken)> = cut_err(
        repeat(
            1..,
            delimited(
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((
                        my_space1.value(()),
                        parse_comment.value(()),
                        line_ending.value(()),
                        ':'.value(())
                    ))
                ),
                (
                    terminated(
                        parse_label(false),
                        alt((((my_space0, ':', my_space0)).take(), my_space1.take()))
                    )
                    .verify(|label: &InnerZ80Span| !label.eq_ignore_ascii_case(b"endstruct"))
                    .context(StrContext::Label("STRUCT: label error"))
                    .map(|span: InnerZ80Span| Z80Span::from(span)),
                    cut_err(
                        parse_struct_directive
                            .context(StrContext::Label("STRUCT: Invalid operation"))
                    )
                ),
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((
                        my_space1.value(()),
                        parse_comment.value(()),
                        line_ending.value(()),
                        ':'.value(())
                    ))
                )
            )
        )
        .context(StrContext::Label("STRUCT: error in inner content"))
    )
    .parse_next(input)?;

    let _ = cut_err(preceded(
        my_space0,
        alt((
            parse_directive_word(b"ENDSTRUCT"),
            parse_directive_word(b"ENDS")
        ))
    ))
    .parse_next(input)?;

    Ok(LocatedTokenInner::Struct(name.into(), fields))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_snaset(
    directive_name_parsed: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedTokenInner, Z80ParserError> {
    move |input: &mut InnerZ80Span| -> ModalResult<LocatedTokenInner, Z80ParserError> {
        if !directive_name_parsed {
            parse_word(b"SNASET").parse_next(input)?;
        }

        let input_start = input.checkpoint();
        let flagname = cut_err(parse_label(false).context(SNASET_WRONG_LABEL)).parse_next(input)?;
        let _ = cut_err(parse_comma.context(SNASET_MISSING_COMMA)).parse_next(input)?;

        let values: Vec<_> = cut_err(separated(
            1..,
            parse_flag_value_inner.context(StrContext::Label("SNASET: wrong flag value")),
            delimited(my_space0, parse_comma, my_space0)
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
            input.reset(&input_start);
            ErrMode::Backtrack(Z80ParserError::from_input(input).add_context(
                input,
                &input_start,
                "Wrong flag"
            ))
        })?;
        Ok(LocatedTokenInner::SnaSet(flag, value))
    }
}

/// Parse a comment that start by `;` and ends at the end of the line.
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_comment(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let cloned = *input;
    preceded(alt((b";", b"//")), take_till(0.., |ch| ch == b'\n'))
        .take()
        .map(|string: &[u8]| {
            LocatedTokenInner::Comment(cloned.update_slice(string).into())
                .into_located_token_direct()
        })
        .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_multiline_comment(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedToken, Z80ParserError> {
    let cloned = *input;
    delimited(b"/*", take_until(0.., "*/"), b"*/")
        .map(|string: &[u8]| {
            LocatedTokenInner::Comment(cloned.update_slice(string).into())
                .into_located_token_direct()
        })
        .parse_next(input)
}

/// TODO
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn string_expr(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    parse_string.map(LocatedExpr::String).parse_next(input)
}

/// Parse a label(label: S)
/// TODO reimplement to never build a string
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_label(
    doubledots: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let start = input.checkpoint();

        let is_orgams = input.state.options().is_orgams();
        let obtained_label = if is_orgams {
            ((
                opt(alt(("::", "@", "."))).value(()),
                alt((
                    one_of((
                        b'a'..=b'z',
                        b'A'..=b'Z',
                        b'_', 
                        b'#', b'\'' // orgams additions
                    )).value(()),
                    delimited('{', expr, '}').value(())
                )),
                repeat::<_, _, (), _, _>(0.., alt((
                    take_while(1..,
                        (b'a'..=b'z',
                        b'A'..=b'Z',
                        b'0'..=b'9',
                        b'_', 
                        b'#', b'\'' // orgams additions
                    )
                      ).value(()),
                    ".".value(()),
                    delimited('{', opt(expr), '}').value(())
                )))
            )).take()
            .parse_next(input)?
        } else {
            ((
                opt(alt(("::", "@", "."))).value(()),
                alt((
                    one_of((
                        b'a'..=b'z',
                        b'A'..=b'Z',
                        b'_', 
                    )).value(()),
                    delimited('{', expr, '}').value(())
                )),
                repeat::<_, _, (), _, _>(0.., alt((
                    take_while(1..,
                        (b'a'..=b'z',
                        b'A'..=b'Z',
                        b'0'..=b'9',
                        b'_', 
                    )
                      ).value(()),
                    ".".value(()),
                    delimited('{', opt(expr), '}').value(())
                )))
            )).take()
            .parse_next(input)?
        };



/*
        // fail to parse a label when it is 100% sure it corresponds to  a macro call
        let (macro_arg) = opt(preceded(space1, Caseless("(void)".into()))).parse_next(input)?;
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
            obtained_label
        };

        //needed because of AT2
        let input = if doubledots {
            let _ =opt(Caseless(":")).parse_next(input)?;
            input
        }
        else {
            input
        };


        // Be sure that ::ld is not considered to be a label
        let label_len = true_label.len();
        if label_len >= MIN_MAX_LABEL_SIZE.0 &&
        label_len <= DOTTED_MIN_MAX_LABEL_SIZE.1 &&
            !ignore_ascii_case_allowed_label( true_label, input.state.options().dotted_directive, input.state.options().assembler_flavor)  {
            input.reset(&start);
            Err(ErrMode::Backtrack(Z80ParserError::from_input(
                input
            ).add_context(input, &start, "You cannot use a directive or an instruction as a label")
            ))
        }
        else {
            Ok((*input).update_slice(obtained_label))
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn impossible_names(dotted_directive: bool, flavor: AssemblerFlavor) -> &'static [&'static [u8]] {
    if flavor == AssemblerFlavor::Basm {
        if dotted_directive {
            &DOTTED_IMPOSSIBLE_NAMES
        }
        else {
            &IMPOSSIBLE_NAMES
        }
    }
    else {
        assert_eq!(flavor, AssemblerFlavor::Orgams);
        &IMPOSSIBLE_NAMES_ORGAMS
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn ignore_ascii_case_allowed_label(
    name: &[u8],
    dotted_directive: bool,
    flavor: AssemblerFlavor
) -> bool {
    #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
    let iter = impossible_names(dotted_directive, flavor).par_iter();

    #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
    let mut iter = impossible_names(dotted_directive, flavor).iter();

    !iter.any(|&content| content.eq_ignore_ascii_case(name))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_end_directive(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    if input.state.options().dotted_directive {
        b'.'.parse_next(input)?;
    }

    if input.state.options().is_orgams() {
        let bracket = opt("]").parse_next(input)?;
        if let Some(bracket) = bracket {
            return Ok((*input).update_slice(bracket));
        }
    }

    let keyword =
        take_while(1.., (b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_')).parse_next(input)?;

    if END_DIRECTIVE
        .iter()
        .any(|&val| val.eq_ignore_ascii_case(keyword))
    {
        Ok((*input).update_slice(keyword))
    }
    else {
        Err(ErrMode::Backtrack(Z80ParserError::from_input(input)))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_macro_name(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let dotted_directive = input.state.options().dotted_directive;
    let flavor = input.state.options().assembler_flavor;

    let name = (
        one_of((b'a'..=b'z', b'A'..=b'Z', b'_')),
        take_while(0.., (b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_')),
        not('{')
    )
        .take()
        .verify(move |name: &[u8]| {
            !(!ignore_ascii_case_allowed_label(name, dotted_directive, flavor))
        })
        .parse_next(input)?;

    Ok((*input).update_slice(name))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn prefixed_label_expr(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let _ = my_space0(input)?;
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let prefix = alt((
        Caseless("{bank}").value(LabelPrefix::Bank),
        Caseless("{page}").value(LabelPrefix::Page),
        Caseless("{pageset}").value(LabelPrefix::Pageset)
    ))
    .parse_next(input)?;

    let label =
        preceded(my_space0, alt((parse_label(false).take(), "$$", "$"))).parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(LocatedExpr::PrefixedLabel(
        prefix,
        (*input).update_slice(label).into(),
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
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_value(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let (val, span) = cpclib_common::parse_value.with_taken().parse_next(input)?;

    let span = (*input).update_slice(span);
    Ok(LocatedExpr::Value(val as i32, span.into()))
}

/// Parse a repetition counter
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_counter(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let cloned = *input;
    delimited(
        b'{',
        parse_label(false), // BUG will accept too many cases
        (b'}', not(alphanumeric1))
    )
    .take()
    .map(|l| LocatedExpr::Label(cloned.update_slice(l).into()))
    .parse_next(input)
}

/// Read a parenthesed expression
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parens(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let (open, close) = if input.state.options().is_orgams() {
        ('[', ']')
    }
    else {
        ('(', ')')
    };

    let exp = delimited(
        delimited(my_space0, open, my_space0),
        located_expr,
        delimited(my_space0, close, my_space0)
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(LocatedExpr::Paren(Box::new(exp), span.into()))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_expr_bracketed_list(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let list = delimited(
        ("[", (my_space0, opt((line_ending, my_space0)))),
        separated(0.., located_expr, parse_comma_multiline),
        ((my_space0, opt((line_ending, my_space0))), "]")
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(LocatedExpr::List(list, span.into()))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_bool_expr(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();
    let bool = alt((
        parse_word(b"true").value(true),
        parse_word(b"false").value(false)
    ))
    .parse_next(input)?;
    let span = build_span(input_offset, &input_start, *input);
    Ok(LocatedExpr::Bool(bool, span.into()))
}

/// Get a factor
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_factor(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let is_orgams = input.state.options().is_orgams();

    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let not = opt(delimited(
        my_space0,
        alt(('!'.take(), parse_word(b"NOT").take())),
        my_space0
    ))
    .parse_next(input)?;

    let binary_not = opt(delimited(my_space0, '~', my_space0)).parse_next(input)?;
    let high_or_low = opt(preceded(my_space0, alt((b'>', b'<')))).parse_next(input)?;

    let cloned = *input;
    let factor = preceded(
        my_space0,
        alt((
            prefixed_label_expr,
            parse_expr_bracketed_list.verify(|_| !is_orgams),
            // Manage functions
            parse_word(b"RND()").map(|w| LocatedExpr::Rnd(w.into())),
            parse_unary_function_call,
            parse_binary_function_call,
            parse_duration,
            parse_assemble,
            parse_any_function_call,
            // manage values
            alt((positive_number, negative_number)),
            parse_string.map(|s| {
                if s.as_ref().len() == 1 {
                    LocatedExpr::Char(s.0.chars().next().unwrap(), s.1)
                }
                else {
                    LocatedExpr::String(s)
                }
            }),
            parse_counter,
            // manage $ and $$
            alt(("$$", "$")).map(|l| LocatedExpr::Label(cloned.update_slice(l).into())),
            (
                "-",
                alt(("$$", "$"))
                    .map(|l| Box::new(LocatedExpr::Label(cloned.update_slice(l).into())))
            )
                .with_taken()
                .map(|((m, dollar), content)| {
                    LocatedExpr::UnaryOperation(
                        UnaryOperation::Neg,
                        dollar,
                        cloned.update_slice(content).into()
                    )
                }),
            parse_bool_expr,
            // manage labels
            parse_label(false).map(|l| LocatedExpr::Label(l.into())),
            parens
        )) /* ,
            * my_space0 */
    )
    .parse_next(input)?;

    // XXX I have replaced Neg by Not, this seems the most coherent stuff
    // XXX Need to check later
    let factor = match not {
        Some(_) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::Not,
                Box::new(factor),
                build_span(input_offset, &input_start, *input).into()
            )
        },
        None => factor
    };

    let factor = match binary_not {
        Some(_) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::BinaryNot,
                Box::new(factor),
                build_span(input_offset, &input_start, *input).into()
            )
        },
        None => factor
    };

    let factor = match high_or_low {
        Some(k) => {
            LocatedExpr::UnaryFunction(
                match k {
                    b'>' => UnaryFunction::High,
                    b'<' => UnaryFunction::Low,
                    _ => unreachable!()
                },
                Box::new(factor),
                build_span(input_offset, &input_start, *input).into()
            )
        },
        None => factor
    };

    Ok(factor)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn negative_number(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let v = preceded(b'-', number)
        .map(|exp| {
            match exp {
                LocatedExpr::Value(v, _) => -v,
                _ => unreachable!()
            }
        })
        .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(LocatedExpr::Value(v, span.into()))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn number(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let _input_start = input.checkpoint();
    let _input_offset = input.eof_offset();

    terminated(
        parse_value,
        not(one_of((
            b'A'..=b'Z',
            b'a'..=b'z',
            b'0'..=b'9',
            b'#',
            b'@',
            b'_'
        )))
    )
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn positive_number(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    preceded(opt('+'), number).parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_labelprefix(input: &mut InnerZ80Span) -> ModalResult<LabelPrefix, Z80ParserError> {
    alt((
        Caseless("{pageset}").value(LabelPrefix::Pageset),
        Caseless("{bank}").value(LabelPrefix::Bank),
        Caseless("{page}").value(LabelPrefix::Page)
    ))
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn fold_exprs(
    initial: LocatedExpr,
    remainder: Vec<(BinaryOperation, LocatedExpr)>,
    span: InnerZ80Span
) -> LocatedExpr {
    remainder.into_iter().fold(initial, move |acc, pair| {
        let (oper, expr) = pair;
        LocatedExpr::BinaryOperation(oper, Box::new(acc), Box::new(expr), span.into())
    })
}

/// Compute operations related to * % /
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn term(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let initial = parse_factor(input)?;
    let remainder = repeat(
        0..,
        alt((
            parse_oper(parse_factor, "*", BinaryOperation::Mul),
            parse_oper(parse_factor, "%", BinaryOperation::Mod),
            parse_oper(parse_factor, "MOD", BinaryOperation::Mod),
            parse_oper(parse_factor, "/", BinaryOperation::Div)
        ))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(fold_exprs(initial, remainder, span))
}

/// Generate a parser of comparison symbol
/// inner: the function the parse the right operand of the symbol
/// pattern: the pattern to match in the source code
/// symbol: the symbol corresponding to the operation
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_oper<F>(
    inner: F,
    pattern: &'static str,
    symbol: BinaryOperation
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<(BinaryOperation, LocatedExpr), Z80ParserError>
where
    F: Fn(&mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError>
{
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let _ = my_space0(input)?;
        let _ = Caseless(pattern).parse_next(input)?;

        // for orgams we cannot accept * as being a mulitplication if it is followed by another * as it repreents a repetition
        if input.state.options().is_orgams() && pattern == "*" {
            not(pattern).parse_next(input)?;
        }

        let _ = my_space0(input)?;
        let operation = inner(input)?;

        Ok((symbol, operation))
    }
}
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
fn parse_bool<F>(
    inner: F,
    pattern: &'static str,
    symbol: BinaryOperation
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<(BinaryOperation, LocatedExpr), Z80ParserError>
where
    F: Fn(&mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError>
{
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let _ = my_space0(input)?;
        let _ = Caseless(pattern).parse_next(input)?;
        let _ = my_space0(input)?;
        let operation = inner(input)?;

        Ok((symbol, operation))
    }
}

/// Parse an expression
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn expr2(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let initial = shift(input)?;
    let remainder = repeat(
        0..,
        alt((
            parse_oper(shift, "<=", BinaryOperation::LowerOrEqual),
            parse_oper(shift, "<", BinaryOperation::StrictlyLower),
            parse_oper(shift, ">=", BinaryOperation::GreaterOrEqual),
            parse_oper(shift, ">", BinaryOperation::StrictlyGreater),
            parse_oper(shift, "==", BinaryOperation::Equal),
            parse_oper(shift, "=", BinaryOperation::Equal),
            parse_oper(shift, "!=", BinaryOperation::Different)
        ))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(fold_exprs(initial, remainder, span))
}

fn expr(input: &mut InnerZ80Span) -> ModalResult<Expr, Z80ParserError> {
    located_expr
        .map(|e| e.to_expr().into_owned())
        .parse_next(input)
}

/// TODO replace ALL expr parse by a located version
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn located_expr(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    if input.state.options().is_orgams() {
        return parse_orgams_expression.parse_next(input);
    }

    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let initial = expr2(input)?;
    let remainder = repeat(
        0..,
        alt((
            parse_oper(expr2, "&&", BinaryOperation::BooleanAnd),
            parse_oper(expr2, "||", BinaryOperation::BooleanOr)
        ))
    )
    .parse_next(input)?;
    let span = build_span(input_offset, &input_start, *input);
    Ok(fold_exprs(initial, remainder, span))
}

/// parse functions with one argument
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_unary_function_call(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let (word, exp) = (
        delimited(my_space0, alpha1, my_space0),
        delimited((my_space0, "(", my_space0), located_expr, (my_space0, ")"))
            .context(StrContext::Label("UNARY function: error in parameters"))
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
        choice_nocase!(b"SQRT") => Some(UnaryFunction::Sqrt),
        choice_nocase!(b"ABS") => Some(UnaryFunction::Abs),
        _ => None
    };

    let span = build_span(input_offset, &input_start, *input);
    let word = (*input).update_slice(word);

    let token = match func {
        Some(func) => LocatedExpr::UnaryFunction(func, Box::new(exp), span.into()),
        None => LocatedExpr::AnyFunction(word.into(), vec![exp], span.into())
    };

    Ok(token)
}

/// parse functions with two arguments
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_binary_function_call(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let func = alt((
        Caseless("MIN").value(BinaryFunction::Min),
        Caseless("MAX").value(BinaryFunction::Max),
        Caseless("POW").value(BinaryFunction::Pow)
    ))
    .parse_next(input)?;

    let _ = ((my_space0, "(", my_space0)).parse_next(input)?;

    let arg1 = located_expr.parse_next(input)?;
    let _ = ((my_space0, ',', my_space0)).parse_next(input)?;
    let arg2 = located_expr.parse_next(input)?;

    let _ = ((my_space0, ")")).parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);

    Ok(LocatedExpr::BinaryFunction(
        func,
        Box::new(arg1),
        Box::new(arg2),
        span.into()
    ))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_any_function_call(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedExpr, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    let function_name = parse_label(false).parse_next(input)?;
    let arguments = delimited(
        (/* space0, */ "(", my_space0),
        separated(0.., located_expr, parse_comma),
        (my_space0, ")")
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(LocatedExpr::AnyFunction(
        function_name.into(),
        arguments,
        span.into()
    ))
}

/// Parser for functions taking into argument a token
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn token_function<'a>(
    function_name: &'static str
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let _ = ((Caseless(function_name), my_space0, ('('), my_space0)).parse_next(input)?;

        let token = parse_token(input)?;

        let _ = ((my_space0, ")")).parse_next(input)?;

        Ok(token)
    }
}

/// Parse the duration function
pub fn parse_duration(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let (token, span) = token_function("duration").with_taken().parse_next(input)?;

    let span = (*input).update_slice(span).into();
    Ok(LocatedExpr::UnaryTokenOperation(
        UnaryTokenOperation::Duration,
        Box::new(token),
        span
    ))
}

/// Parse the single opcode assembling function
pub fn parse_assemble(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let (token, span) = token_function("opcode").with_taken().parse_next(input)?;

    let span = (*input).update_slice(span).into();
    Ok(LocatedExpr::UnaryTokenOperation(
        UnaryTokenOperation::Opcode,
        Box::new(token),
        span
    ))
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn shift(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let start = input.checkpoint();
    let start_eof_offset = input.eof_offset();

    let initial = comp(input)?;
    let remainder = repeat(
        0..,
        alt((
            parse_oper(comp, "<<", BinaryOperation::LeftShift),
            parse_oper(comp, ">>", BinaryOperation::RightShift)
        ))
    )
    .parse_next(input)?;

    Ok(fold_exprs(
        initial,
        remainder,
        build_span(start_eof_offset, &start, *input)
    ))
}

/// Parse operation related to + - & |
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn comp(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let start = input.checkpoint();
    let start_eof_offset = input.eof_offset();

    let initial = term(input)?;
    let remainder = repeat(0.., alt((
        parse_oper(term, "+", BinaryOperation::Add),
        parse_oper(term, "-", BinaryOperation::Sub),
        parse_oper(term, "&", BinaryOperation::BinaryAnd), /* TODO check if it works and not compete with && */
        parse_oper(term, "AND", BinaryOperation::BinaryAnd),
        parse_oper(term, "|", BinaryOperation::BinaryOr), /* TODO check if it works and not compete with || */
        parse_oper(term, "OR", BinaryOperation::BinaryOr),
        parse_oper(term, "^", BinaryOperation::BinaryXor), /* TODO check if it works and not compete with ^^ */
        parse_oper(term, "XOR", BinaryOperation::BinaryXor)
    ))).parse_next(input)?;

    Ok(fold_exprs(
        initial,
        remainder,
        build_span(start_eof_offset, &start, *input)
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
            assert!(
                unsafe { std::str::from_utf8_unchecked(span.0.as_bstr()) }
                    .trim_start()
                    .starts_with(next)
            );
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

        let res = parse_test(
            parse_assembler_control_max_passes_number,
            "ASMCONTROLENV SET_MAX_NB_OF_PASSES=10: nop : ENDA"
        );
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
            (parse_conditional, line_ending, my_space1),
            "if THING
                    nop
                    endif
                    "
        );
        assert!(res.is_ok());

        let res = parse_test(
            parse_conditional,
            "ifnot 5
print glop
else
endif"
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
    fn test_parse_run() {
        let res: TestResult<LocatedTokenInner> = parse_test(parse_run(RunEnt::Run), "0x50, 0xc0");
        assert!(res.is_ok(), "{:?}", &res);
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
    fn test_parse_advanced_breakpoints() {
        assert!(dbg!(parse_test(parse_argname_to_assign("TYPE"), "TYPE=")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint_type_value, "mem")).is_ok());
        assert!(
            dbg!(parse_test(
                parse_argname_and_value("TYPE", &parse_breakpoint_type_value),
                "TYPE=mem"
            ))
            .is_ok()
        );

        assert!(
            dbg!(parse_test(
                parse_optional_argname_and_value("TYPE", &parse_breakpoint_type_value),
                "TYPE=mem"
            ))
            .is_ok()
        );
        assert!(
            dbg!(parse_test(
                parse_optional_argname_and_value("TYPE", &parse_breakpoint_type_value),
                "TYPE = mem"
            ))
            .is_ok()
        );

        assert!(dbg!(parse_test(parse_breakpoint_argument, "mem")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint_argument, "read")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint_argument, "TYPE=mem")).is_ok());

        // breakpoint keyword has alrady been consumed
        assert!(dbg!(parse_test(parse_breakpoint, "")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "address")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "mem")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "TYPE=mem")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "ACCESS=READ")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "READ")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "RUNMODE=STOP")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "ADDR=here")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "MASK=12")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "SIZE=1")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "VALUE=1")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "VALMASK=1")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "condition=\"fdfdfd\"")).is_ok());
        assert!(dbg!(parse_test(parse_breakpoint, "name=\"fdfdfd\"")).is_ok());

        assert!(dbg!(parse_test(parse_breakpoint, "step=10,name=\"fdfdfd\"")).is_ok());
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
    fn test_parse_list() {
        let res = parse_test(parse_expr_bracketed_list, "[0, 1]");
        assert!(res.is_ok(), "{:?}", res);

        let res = parse_test(
            parse_expr_bracketed_list,
            "[0, \
        1]"
        );
        assert!(res.is_ok(), "{:?}", res);

        let res = parse_test(
            parse_expr_bracketed_list,
            "[0,
        1]"
        );
        assert!(res.is_ok(), "{:?}", res);

        let res = parse_test(
            parse_expr_bracketed_list,
            "[
        0,
        1]"
        );
        assert!(res.is_ok(), "{:?}", res);

        let res = parse_test(
            parse_expr_bracketed_list,
            "[
        0,
        1
        ]"
        );
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
        let res = parse_test(
            parse_assert,
            "assert (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)"
        );
        assert!(res.is_ok(), "{:?}", &res);
    }

    #[test]
    fn parser_macro_fap_bug1() {
        let code = "MACRO   _UpdateNrCopySlot               ; 4 NOPS
        ld	b, a
        ld	a, c
        sub	b
        ld	c, a
MEND";

        let res = parse_test(parse_macro, code);

        assert!(dbg!(&res).is_ok());
        let res = res.as_ref().unwrap();
        let macro_args = dbg!(res.macro_definition_arguments());
        assert_eq!(0, macro_args.len());
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
        res.res.unwrap();

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

        let res = dbg!(parse_test(parse_line(&mut tokens), " hello /* :  world*/"));
        dbg!(&tokens);

        assert!(res.is_ok(), "{:?}", &res);
        assert!(!tokens[0].is_call_macro_or_build_struct());
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
    fn test_parse_multiline_comment() {
        let res = parse_test(parse_multiline_comment, "/* fdfsdfgd */");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_multiline_comment, "/* fdf\n*\n*\nsdfgd */");
        assert!(res.is_ok(), "{:?}", &res);
    }

    #[test]
    fn test_parse_ticker() {
        let res = parse_test(parse_stable_ticker_start, "start mc");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_stable_ticker_start, "start, mc");
        assert!(res.is_ok(), "{:?}", &res);
    }
    #[test]
    fn test_parse_line_component() {
        let res = parse_test(parse_line_component, "ticker start, mc");
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(parse_line_component, "JP HL_div_2");
        assert!(res.is_ok(), "{:?}", &res);

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
            (parse_line_component, my_space1, parse_comment),
            "data1 SETN data ; comment"
        );
        assert!(res.is_ok(), "{:?}", &res);

        let res = parse_test(
            (parse_line_component, my_space1, parse_comment),
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
        assert!(
            res.as_ref().unwrap().1.as_ref().unwrap().is_assign(),
            "{:?}",
            &res
        );
    }

    #[test]
    fn test_parse_marco_arg() {
        assert_eq!(
            parse_test(parse_macro_arg, "arg")
                .as_ref()
                .unwrap()
                .to_macro_param(),
            MacroParam::RawArgument("arg".into())
        );
        assert_eq!(
            parse_test(parse_macro_arg, "{eval}arg")
                .as_ref()
                .unwrap()
                .to_macro_param(),
            MacroParam::EvaluatedArgument("arg".into())
        );
    }

    #[test]
    fn test_parse_label() {
        assert!(dbg!(parse_test(parse_label(false), "HL_div_2")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "CHECK")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "label")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "label.label")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "label{after}")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "{before}label")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "la{inner}bel")).is_ok());
        assert!(dbg!(parse_test(parse_label(false), "label{i+5}")).is_ok());

        assert!(dbg!(parse_test(parse_label(false), "_JP")).is_ok());
    }

    #[test]
    fn test_parse_macro_call() {
        assert!(dbg!(parse_test(parse_line_component, "empty     (void)")).is_ok());

        let res = dbg!(parse_test(
            (parse_line_component, ':', parse_line_component),
            "empty (void):ld a,1"
        ))
        .res
        .unwrap();

        assert!(res.0.0.is_none());
        assert!(res.0.1.is_some());
        assert!(res.2.0.is_none());
        assert!(res.2.1.is_some());

        assert!(
            dbg!(parse_test(
                parse_line_component,
                "notempty \"arg1\", \"arg2\""
            ))
            .is_ok()
        );
    }

    #[test]
    fn test_regression_check() {
        let check = "CHECK";

        let (ctx, mut span) = ctx_and_span("CHECK");
        assert!(dbg!(parse_factor.parse_next(&mut span.0)).is_ok());

        assert!(dbg!(parse_test(parse_label(false), check)).is_ok());
        assert!(dbg!(parse_test(parse_factor, check)).is_ok());
    }

    #[test]
    fn test_parse_expr() {
        for code in &[
            "(2 - $b06e) and $ff",
            "'o'",
            "'o' + 0x80",
            "CHECK",
            "\"\\\" et voila\"",
            "0X1234",
            "<0X1234",
            ">0X1234",
            "TOTO",
            "_TOTO"
        ] {
            assert!(dbg!(parse_test(parse_expr, code)).is_ok());

            assert!(dbg!(parse_test(expr_list, code)).is_ok());
        }
    }

    #[test]
    fn debug_label_expression() {
        for code in &["TOTO", "_TOTO", "_JP"] {
            assert!(dbg!(parse_test(parse_label(false), code)).is_ok());
            assert!(dbg!(parse_test(parse_factor, code)).is_ok());
            assert!(dbg!(parse_test(term, code)).is_ok());
            assert!(dbg!(parse_test(comp, code)).is_ok());
            assert!(dbg!(parse_test(shift, code)).is_ok());
            assert!(dbg!(parse_test(expr2, code)).is_ok());
            assert!(dbg!(parse_test(located_expr, code)).is_ok());
            assert!(dbg!(parse_test(expr, code)).is_ok());
        }
    }

    #[test]
    fn regression_parse_hl() {
        for code in &mut [
            "ld hl, TOTO",
            "ld HL, _TOTO",
            "ld hl, _JP",
            "ld a, TOTO",
            "ld a, _TOTO",
            "ld a, _JP",
            "ld a,_JP"
        ] {
            dbg!("Handle", &code);
            dbg!("parse_ld");
            assert!(dbg!(parse_test(parse_ld(false), code)).is_ok());
            dbg!("parse_instruction");
            assert!(dbg!(parse_test(parse_token, code)).is_ok());
            dbg!("parse_line");
            let mut tokens = Vec::new();
            assert!(dbg!(parse_test(parse_line(&mut tokens), code)).is_ok());
        }
    }

    // TODO find why this test fails wheras cpclib_common::tests::parse_string succeed. I do not get the differences
    #[test]
    fn test_parse_string() {
        for string in &[
            r#""\" et voila""#,
            r#""kjkjhkl""#,
            r#""kjk'jhkl""#,
            r#""kj\"kjhkl""#,
            r#"'kjkjhkl'"#,
            r#"'kjk\\"jhkl'"#,
            r#"'kjkj\'hkl'"#,
            r#""""#,
            r#"''"#,
            r#""fdfd\" et voila""#
        ] {
            let res = parse_test(parse_string, string);
            assert!(dbg!(&res).is_ok());

            assert_eq!(
                res.res.unwrap().1.as_bstr(),
                (&string[1..string.len() - 1]).as_bstr()
            );

            assert!(dbg!(parse_test(parse_factor, string)).is_ok());

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

    #[test]
    fn test_bitwise_or() {
        let res = dbg!(parse_test(expr, "1|2"));
        let res = res.as_ref().unwrap();
        match res {
            Expr::BinaryOperation(BinaryOperation::BinaryOr, ..) => {},
            _ => panic!("Wrong operation")
        }
    }

    #[test]
    fn test_fname() {
        assert!(parse_test(parse_fname, "\"test.asm\"").is_ok());
        assert!(parse_test(parse_fname, "test.asm").is_ok());
        assert!(dbg!(parse_test(parse_fname, "src/credits_screen.asm")).is_ok());

        assert!(parse_test(parse_directive, "include \"test.asm\"").is_ok());
        assert!(parse_test(parse_directive, "include test.asm").is_ok());
        assert!(parse_test(parse_directive, "include good_db.asm").is_ok());
        assert!(parse_test(parse_include, "good_db.asm").is_ok());
        assert!(dbg!(parse_test(parse_include, "src/credits_screen.asm")).is_ok());

        assert!(dbg!(parse_test((parse_directive, "  "), "incbin \"test.asm\"  ")).is_ok());
        assert!(parse_test((parse_directive, "  "), "incbin test.asm  ").is_ok());
    }
}
