#![allow(clippy::cast_lossless)]

use std::convert::{TryFrom, TryInto};
use std::ops::Deref;
use std::sync::{Arc};

use cpclib_common::itertools::{chain, Itertools};
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
use cpclib_common::rayon::prelude::*;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::{bin_number, dec_number, hex_number, lazy_static};
use cpclib_sna::parse::{parse_flag, parse_flag_value};
use cpclib_sna::{FlagValue, SnapshotVersion};

use super::context::*;
use super::obtained::*;
use super::*;
use crate::preamble::*;

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
    "DS",
    "DW",
    "ELSE",
    "EXPORT",
    "FAIL",
    "INCBIN",
    "INCLUDE",
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
    "TICKER",
    "UNDEF",
    "UNTIL",
    "WAITNOPS",
    "WORD",
    "WRITE DIRECT",
    "WRITE"
];

const START_DIRECTIVE: &[&str] = &[
    "FOR", "IF", "IFDEF", "IFNDEF", "IFUSED", "ITER", "ITERATE", "LZ4", "LZ48", "LZ49", "LZAPU",
    "LZEXO", "LZX7", "MACRO", "MODULE", "PHASE", "REPEAT", "REPT", "STRUCT", "SWITCH", "WHILE"
];

// This table is supposed to contain the keywords that finish a section
const END_DIRECTIVE: &[&str] = &[
    "BREAK",
    "CASE",
    "DEFAULT",
    "DEPHASE",
    "ELSE",
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
}

pub fn parse_z80_strrc_with_contextrc(
    code: Arc<String>,
    ctx: Arc<ParserContext>
) -> Result<LocatedListing, AssemblerError> {
    let span = Z80Span::new_extra_from_rc(code, ctx);
    let listing = LocatedListing::new_empty_span(span);
    let ctx = listing.ctx();
    match parse_z80_code(listing.span()) {
        Err(e) => {
            match e {
                cpclib_common::nom::Err::Error(e) | Err::Failure(e) => {
                    return Err(AssemblerError::SyntaxError { error: e });
                }
                cpclib_common::nom::Err::Incomplete(_) => {
                    return Err(AssemblerError::BugInParser {
                        error: "Bug in the parser".to_owned(),
                        context: ctx.deref().clone()
                    });
                }
            }
        }

        Ok((remaining, mut parsed)) => {
            if remaining.len() > 0 {
                return Err(AssemblerError::BugInParser {
                    error: format!(
                        "Bug in the parser. The remaining source has not been assembled:\n{}",
                        remaining.deref()
                    ),
                    context: ctx.deref().clone()
                });
            }

            /* this feature is currently disabled. Will see later how to implement it properly
            if ctx.read_referenced_files {
                let errors = parsed
                    .listing_mut()
                    .par_iter_mut()
                    .map(|token| token.read_referenced_file(&ctx))
                    .filter(Result::is_err)
                    .map(Result::err)
                    .map(Option::unwrap)
                    .collect::<Vec<_>>();
                if errors.len() > 0 {
                    return Err(AssemblerError::MultipleErrors { errors });
                }
            }
        */

            return Ok(parsed);
        }
    }
}

/// Produce the stream of tokens. In case of error, return an explanatory string.
/// In case of success loop over all the tokens in order to expand those that read files
pub fn parse_z80_str_with_context<S: Into<String>>(
    str: S,
    ctx: ParserContext
) -> Result<LocatedListing, AssemblerError> {
    parse_z80_strrc_with_contextrc(Arc::new(str.into()), Arc::new(ctx))
}

/// Parse a string and return the corresponding listing
pub fn parse_z80_str<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_str_with_context(code, Default::default())
}

/// nom many0 does not seem to fit our parser requirements
pub fn my_many0<O, E, F>(mut f: F) -> impl FnMut(Z80Span) -> IResult<Z80Span, Vec<O>, E>
where
    F: Parser<Z80Span, O, E>,
    E: ParseError<Z80Span>
{
    move |mut i: Z80Span| {
        let mut acc = Vec::with_capacity(4);
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

/// Parse a complete code from a span
pub fn parse_z80_code<'src, 'ctx, 'a>(
    input: Z80Span
) -> IResult<Z80Span, LocatedListing, VerboseError<Z80Span>> {
    let (input, tokens) = fold_many0(
        parse_z80_line,
        || Vec::new(),
        |mut source_tokens, mut line_tokens| {
            source_tokens.append(&mut line_tokens);
            source_tokens
        }
    )(input)?; // here it is my_many0 supposed to be used

    if input.trim().is_empty() {
        Ok((
            input.clone(),
            tokens
                .try_into()
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input))
        ))
    }
    else {
        // Everything should have been consumed
        return Err(Err::Error(
            cpclib_common::nom::error::ParseError::<Z80Span>::from_error_kind(
                input,
                ErrorKind::Many0
            )
        ));
    }
}

/// Parse a single line of Z80. Code useing directive on several lines cannot work
pub fn parse_z80_line(
    input: Z80Span
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    let before_elem = input.clone();
    let (input2, tokens) = tuple((
        context("[DBG]not eof", not(eof)),
        alt((
            context("[DBG] empty line", parse_empty_line),
            context(
                "[DBG] code embeders",
                delimited(
                    space0,
                    alt((
                        map(context("basic", parse_basic), |t| {
                            vec![t.locate(before_elem.clone())]
                        }),
                        map(
                            context(
                                "block instruction",
                                alt((
                                    context("macro definition", parse_macro),
                                    context("[DBG] crunched section", parse_crunched_section),
                                    context("[DBG] module", parse_module),
                                    context("[DBG] repeat", parse_repeat),
                                    context("[DBG] for", parse_for),
                                    context("Function definition", parse_function),
                                    context("SWITCH parse error", parse_switch),
                                    context("[DBG] iterate", parse_iterate),
                                    context("[DBG] while", parse_while),
                                    context("[DBG] rorg", parse_rorg),
                                    context("[DBG] condition", parse_conditional)
                                ))
                            ),
                            |lt| vec![lt]
                        )
                    )),
                    cut(context(
                        "Line ending issue",
                        preceded(
                            space0,
                            alt((
                                map(parse_comment, |_| "".into()),
                                line_ending,
                                eof,
                                tag(":")
                            ))
                        )
                    ))
                )
            ),
            context("[DBG] line with label only", parse_z80_line_label_only),
            context("[DBG] standard line", parse_z80_line_complete)
        ))
    ))(input)?;

    Ok((input2, tokens.1))
}

/// Workaround because many0 is not used in the main root function
fn inner_code(mut input: Z80Span) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    let mut tokens = Vec::new();
    loop {
        // check if the line need to be parsed (ie there is no end directive)
        let must_break = input.trim().is_empty() || {
            // TODO take into account potential label
            let maybe_keyword = opt(preceded(
                delimited(space0, opt(tag(":")), space0),
                parse_end_directive
            ))(input.clone());
            match maybe_keyword {
                Ok((_, Some(_))) => true,
                _ => false
            }
        };
        if must_break {
            break;
        };

        // really parse the line
        let (line_input, mut tok) = cut(context("[DBG] Inner loop", parse_z80_line))(input)?;
        input = line_input;
        tokens.append(&mut tok);
    }

    Ok((input, tokens))
}

/// TODO
pub fn parse_rorg(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let (input, _) = space0(input)?;
    let rorg_start = input.clone();
    let (input, _) = alt((tag_no_case("PHASE"), tag_no_case("RORG")))(input)?;

    let (input, exp) = delimited(space1, expr, space0)(input)?;

    let (input, _) = line_ending(input)?;

    let (input, inner) = inner_code(input)?;

    let (input, _) = preceded(space0, alt((tag_no_case("DEPHASE"), tag_no_case("REND"))))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Rorg(
            exp,
            inner
                .try_into()
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            rorg_start
        )
    ))
}

/// TODO - limit the listing possibilities
pub fn parse_function_listing(
    input: Z80Span
) -> IResult<Z80Span, LocatedListing, VerboseError<Z80Span>> {
    let input = input.clone_with_state(ParsingState::FunctionLimited);
    let (output, inner) = inner_code(input.clone())?;
    let inner = inner.try_into().unwrap_or_else(|_| {
        LocatedListing::new_empty_span(input.slice(..input.len() - output.len()))
    });
    Ok((output.clone_with_state(ParsingState::Standard), inner))
}

pub fn parse_function(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
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
                    take_till(|c| c == '\n' || c == '\r' || c == ':' || c == ',' || c == ' '),
                    my_space0
                )
            )
        )
    ))(input)?;

    let (input, _) = preceded(space0, alt((line_ending, tag(":"))))(input)?;
    let (before_expr, listing) =
        cut(context("FUNCTION: invalid content", parse_function_listing))(input)?;

    let (input, _) = many0(alt((space1, line_ending, tag(":"))))(before_expr)?;
    let (input, _) = alt((
        parse_directive_word("ENDF"),
        parse_directive_word("ENDFUNCTION")
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Function(
            name,
            arguments
                .iter()
                .map(|a| SmolStr::from_iter(a.fragment().chars()))
                .collect_vec(),
            listing,
            function_start.slice(..function_start.len() - input.len())
        )
    ))
}

/// TODO
pub fn parse_macro(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
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

    let (input, content) = cut(context(
        "MACRO: issue in the content",
        preceded(
            alt((space0, line_ending, tag(":"))),
            many_till(
                take(1usize),
                alt((
                    parse_directive_word("ENDM"),
                    parse_directive_word("ENDMACRO"),
                    parse_directive_word("MEND")
                ))
            )
        )
    ))(input)?;

    Ok((
        input.clone(),
        Token::Macro(
            name,
            arguments
                .iter()
                .map(|s| SmolStr::from_iter(s.fragment().chars()))
                .collect::<Vec<SmolStr>>(),
            content
                .0
                .iter()
                .map(|s| -> String { s.to_string() })
                .collect::<String>()
        )
        .locate(Z80Span(unsafe {
            LocatedSpan::new_from_raw_offset(
                dir_start.location_offset(),
                dir_start.location_line(),
                &dir_start[..dir_start.len() - input.len()],
                dir_start.extra.clone()
            )
        }))
    ))
}

/// TODO
pub fn parse_while(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let while_start = input.clone();
    let (input, _) = parse_directive_word("WHILE")(input)?;

    let (input, cond) = cut(context("WHILE: error in condition", expr))(input)?;
    let (input, inner) = cut(context("WHILE: issue in the content", inner_code))(input)?;
    let (input, _) = cut(context(
        "WHILE: not closed",
        preceded(
            space0,
            alt((parse_directive_word("ENDW"), parse_directive_word("WEND")))
        )
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::While(
            cond,
            LocatedListing::try_from(inner)
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            while_start
        )
    ))
}

pub fn parse_module(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
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
        LocatedToken::Module(
            name,
            LocatedListing::try_from(inner)
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            module_start
        )
    ))
}

/// Parse a sub-listing part that aims at being crunched after being assembled at first pass
pub fn parse_crunched_section(
    input: Z80Span
) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let crunched_start = input.clone();
    let (input, kind) = preceded(
        space0,
        alt((
            map(parse_directive_word("LZEXO"), |_| CrunchType::LZEXO),
            map(parse_directive_word("LZ4"), |_| CrunchType::LZ4),
            map(parse_directive_word("LZ48"), |_| CrunchType::LZ48),
            map(parse_directive_word("LZ49"), |_| CrunchType::LZ49),
            map(parse_directive_word("LZX7"), |_| CrunchType::LZX7),
            map(parse_directive_word("LZAPU"), |_| CrunchType::LZAPU)
        ))
    )(input)?;

    let (input, inner) = cut(context(
        "CRUNCHED SECTION: issue in the content",
        inner_code
    ))(input)?;

    let (input, _) = cut(context(
        "REPEAT: not closed",
        tuple((space0, parse_directive_word("LZCLOSE"), space0))
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::CrunchedSection(
            kind,
            LocatedListing::try_from(inner)
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            crunched_start
        )
    ))
}

/// Parse the switch directive
pub fn parse_switch(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let (switch_start, _) = many0(alt((space1, line_ending)))(input)?;
    let (input, _) = parse_directive_word("SWITCH")(switch_start.clone())?;

    let (input, value) = cut(context("SWITCH: tested value", preceded(space0, expr)))(input)?;

    let mut cases_listing = Vec::new();
    let mut default_listing = None;

    let mut loop_start = input;
    loop {
        let (input, _) = cut(context(
            "SWITCH: whitespace error",
            many0(alt((
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
                delimited(space0, expr, opt(tag(":")))
            ))(input)?;

            let (input, inner) = cut(context("SWITCH: error in case code", inner_code))(input)?;

            let (input, do_break) = opt(preceded(space0, parse_directive_word("BREAK")))(input)?;

            cases_listing.push((
                value,
                LocatedListing::try_from(inner)
                    .unwrap_or_else(|_| LocatedListing::new_empty_span(input.clone())),
                do_break.is_some()
            ));
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
            default_listing = Some(
                LocatedListing::try_from(default)
                    .unwrap_or_else(|_| LocatedListing::new_empty_span(input.clone()))
            );
            input
        }
    }
}

pub fn parse_for(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let for_start = input.clone();
    let (input, _) = preceded(space0, parse_directive_word("FOR"))(input)?;

    // Get parameters
    let (input, counter) = cut(parse_label(false))(input)?;
    let (input, start) = cut(preceded(parse_comma, expr))(input)?;
    let (input, stop) = cut(preceded(parse_comma, expr))(input)?;
    let (input, step) = opt(preceded(parse_comma, expr))(input)?;

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
            listing: LocatedListing::try_from(inner)
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            span: for_span
        }
    ))
}

pub fn parse_repeat(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let repeat_start = input.clone();
    let (input, _) = preceded(
        space0,
        alt((
            parse_directive_word("REP"),
            parse_directive_word("REPT"),
            parse_directive_word("REPEAT")
        ))
    )(input)?;

    let (input, count) = opt(expr)(input)?;
    match count {
        Some(count) => {
            let (input, counter) = cut(context(
                "REPEAT: issue in the counter",
                opt(preceded(parse_comma, parse_label(false)))
            ))(input)?;
            let (input, counter_start) = opt(preceded(parse_comma, expr))(input)?;
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
                    LocatedListing::try_from(inner)
                        .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
                    counter,
                    counter_start,
                    repeat_start
                )
            ))
        }

        None => {
            let (input, inner) = cut(context("REPEAT: issue in the content", inner_code))(input)?;

            let (input, _) = cut(context(
                "REPEAT ... UNTIL: not closed",
                delimited(space0, parse_directive_word("UNTIL"), space0)
            ))(input)?;
            let (input, cond) = cut(context("REPEAT UNTIL: condition error", expr))(input)?;
            Ok((
                input.clone(),
                LocatedToken::RepeatUntil(
                    cond,
                    LocatedListing::try_from(inner)
                        .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
                    repeat_start
                )
            ))
        }
    }
}

pub fn parse_iterate(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
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
                map(parse_label(false), Expr::Label)
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
        LocatedToken::Iterate(
            counter,
            values,
            LocatedListing::try_from(inner)
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            iterate_start
        )
    ))
}

/// TODO
pub fn parse_basic(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
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

    Ok((input, Token::Basic(args, hidden_lines, basic.to_string())))
}

/// Parse the instruction to hide basic lines
pub fn parse_basic_hide_lines(input: Z80Span) -> IResult<Z80Span, Vec<u16>, VerboseError<Z80Span>> {
    let (input, _) = tuple((tag_no_case("HIDE_LINES"), space1))(input)?;
    separated_list1(
        preceded(space0, char(',')),
        preceded(space0, map(dec_number_inner, |d| d as u16))
    )(input)
}
pub fn dec_number_inner(input: Z80Span) -> IResult<Z80Span, u32, VerboseError<Z80Span>> {
    let input_inner = input.deref().clone();
    let (input, number) = dec_number(input_inner).map_err(|_err| {
        cpclib_common::nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric
        ))
    })?;

    Ok((Z80Span(input), number))
}
pub fn bin_number_inner(input: Z80Span) -> IResult<Z80Span, u32, VerboseError<Z80Span>> {
    let input_inner = input.deref().clone();
    let (input, number) = bin_number(input_inner).map_err(|_err| {
        cpclib_common::nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric
        ))
    })?;

    Ok((Z80Span(input), number))
}
pub fn hex_number_inner(input: Z80Span) -> IResult<Z80Span, u32, VerboseError<Z80Span>> {
    let input_inner = input.deref().clone();
    let (input, number) = hex_number(input_inner).map_err(|_err| {
        cpclib_common::nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric
        ))
    })?;

    Ok((Z80Span(input), number))
}

pub fn parse_flag_value_inner(
    input: Z80Span
) -> IResult<Z80Span, FlagValue, VerboseError<Z80Span>> {
    let inner_input = input.deref().clone();

    let (input, number) = parse_flag_value(inner_input).map_err(|_err| {
        cpclib_common::nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric
        ))
    })?;

    Ok((Z80Span(input), number))
}

/// TODO - currently consume several lines. Should do it only one time
pub fn parse_empty_line(
    input: Z80Span
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    // let (input, _) = opt(line_ending)(input)?;
    let before_comment = input.clone();
    let (input, comment) = delimited(space0, opt(parse_comment), space0)(input)?;
    let (input, _) = alt((line_ending, eof))(input)?;

    let mut res = Vec::new();
    if comment.is_some() {
        res.push(comment.unwrap().locate(before_comment));
    }

    Ok((input, res))
}

fn parse_single_token(
    first: bool
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    move |input: Z80Span| {
        // Do not match ':' for the first case
        let input = if first {
            input
        }
        else {
            let (input, _) =
                context("[DBG] delimitation", delimited(space0, char(':'), space0))(input)?;
            input
        };

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
}

fn eof(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    if input.len() == 0 {
        Ok((input.clone(), input))
    }
    else {
        Err(Err::Error(error_position!(input, ErrorKind::Eof)))
    }
}

#[derive(Clone, Copy)]
enum LabelModifier {
    Equ,
    Set,
    Equal,
    SetN,
    Next
}

/// Parse a line
/// TODO add an argument o manage cases like '... : ENDIF'
pub fn parse_z80_line_complete(
    input: Z80Span
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    // Eat previous line ending
    let (input, _) = opt(line_ending)(input)?;

    // Eat optional label (or macro call)
    let before_label = input.clone();

    let (input, r#let) = opt(delimited(space0, parse_directive_word("LET"), space0))(input)?;

    // Guess the kind of label previously acquired
    let (input, label_or_macro) = opt(preceded(space0, parse_label(false)))(input)?;
    let (mut input, (mut i_know_it_is_a_label, label_modifier)) = if label_or_macro.is_some() {
        context(
            "check label  modifier",
            alt((
                value((true, None), tuple((space0, char(':'), space0))),
                value(
                    (true, Some(LabelModifier::Set)),
                    tuple((space0, char('='), space0))
                ),
                value(
                    (true, Some(LabelModifier::Equ)),
                    tuple((space0, parse_directive_word("DEFL"), space0))
                ),
                value(
                    (true, Some(LabelModifier::Equ)),
                    tuple((space0, parse_directive_word("EQU"), space0))
                ),
                value(
                    (true, Some(LabelModifier::SetN)),
                    tuple((space0, parse_directive_word("SETN"), space0))
                ),
                value(
                    (true, Some(LabelModifier::Next)),
                    tuple((space0, parse_directive_word("NEXT"), space0))
                ),
                value(
                    (true, Some(LabelModifier::Set)),
                    delimited(
                        space0,
                        parse_directive_word("SET"),
                        not(tuple((space0, expr, parse_comma)))
                    )
                ), // disambiguate with SET mnemonic
                value((false, None), space1)
            ))
        )(input)?
    }
    else {
        (input, (false, None))
    };

    // ensure let uses =
    if r#let.is_some() {
        if let Some(LabelModifier::Equal) = &label_modifier {
            // ok
        }
        else {
            return Err(cpclib_common::nom::Err::Failure(
                VerboseError::from_error_kind(before_label, ErrorKind::Char)
            ));
        }
    }

    // Get the missing information for the standard label
    let (input, expr_arg) = match &label_modifier {
        Some(LabelModifier::Equ | LabelModifier::Equal | LabelModifier::Set) => {
            cut(context("Value error", map(expr, |e| Some(e))))(input)?
        }
        _ => (input, None)
    };
    // Get the target label for the enumeration like label
    let (input, source_label) = match &label_modifier {
        Some(LabelModifier::Next | LabelModifier::SetN) => {
            cut(context(
                "Label expected",
                map(parse_label(false), |l| Some(l))
            ))(input)?
        }
        _ => (input, None)
    };
    // optional expression to control the displacement
    let (input, additional_arg) = match &label_modifier {
        Some(LabelModifier::Next | LabelModifier::SetN) => opt(preceded(parse_comma, expr))(input)?,
        _ => (input, None)
    };

    let mut tokens = Vec::new();

    let input = if label_modifier.is_some() {
        let label = label_or_macro.unwrap();
        // Here we know it was a modified label; so we handle it before treating next opcode
        let token = match label_modifier {
            Some(LabelModifier::Equ) => Token::Equ(label, expr_arg.unwrap()),
            Some(LabelModifier::Equal | LabelModifier::Set) => {
                Token::Assign(label, expr_arg.unwrap())
            }
            Some(LabelModifier::SetN) => Token::SetN(label, source_label.unwrap(), additional_arg),
            Some(LabelModifier::Next) => Token::Next(label, source_label.unwrap(), additional_arg),
            None => Token::Label(label)
        };
        tokens.push(token.locate(before_label));
        input
    }
    else {
        // Here we know it was not a modified label; the following can be a macro or any instruction

        // Try to parse a macro if it is not a label
        let mut i_know_it_is_a_macro = false;
        let nb_warnings = input.extra.1.warnings().len();

        let (input, r#macro) = if label_or_macro.is_some() && !i_know_it_is_a_label {
            match context(
                "MACRO or struct call",
                preceded(space0, parse_macro_or_struct_call(false, false))
            )(before_label.clone())
            {
                Ok((input2, r#macro)) => {
                    // check if there is nothing usefull after
                    let res: IResult<Z80Span, _, VerboseError<Z80Span>> =
                        preceded(space0, alt((line_ending, tag(":"), tag(";"))))(input2.clone());
                    if res.is_ok() {
                        i_know_it_is_a_macro = true;
                        (
                            input2,
                            Some(LocatedToken::Standard {
                                token: r#macro,
                                span: before_label.clone()
                            })
                        )
                        // we know it is a macro and not a label, so we can stop here
                    }
                    else {
                        i_know_it_is_a_label = true;
                        (input, None)
                    }
                }
                Err(_) => {
                    // no change of input that is still after the label
                    i_know_it_is_a_label = true;
                    (input, None)
                }
            }
        }
        else {
            (input, None)
        };

        if i_know_it_is_a_label {
            // remove the unwanted  warnings
            while nb_warnings < input.extra.1.warnings().len() {
                input.extra.1.pop_warning();
            }
        }

        // We add first token as a label or macro
        if label_or_macro.is_some() && i_know_it_is_a_label {
            tokens.push(Token::Label(label_or_macro.clone().unwrap()).locate(before_label.clone()));
        }
        else if r#macro.is_some() {
            tokens.push(r#macro.unwrap());
        }
        else {
            assert!(label_or_macro.is_none())
        }

        // here either we detected a label, either there was no macro
        // and we want to add the corresponding opcode
        if !i_know_it_is_a_macro {
            // input is after the label if any
            let (input2, _) = cut(context(
                "Parse issue, no end directive expected here",
                not(parse_forbidden_keyword)
            ))(input)?;

            // label/macro instruction?
            let nb_warnings = input2.extra.1.warnings().len();
            // try to parse a token. if it fails, fall back to the macro call parser
            let (input2, opcode) = match parse_single_token(true)(input2.clone()) {
                Ok((
                    _input3,
                    LocatedToken::Standard {
                        token: Token::Label(_),
                        ..
                    }
                )) => {
                    // we cannot have a label; it corresponds to a macro call
                    while nb_warnings < input2.extra.1.warnings().len() {
                        input2.extra.1.pop_warning();
                    }
                    let (input3, macro_call) = cut(context(
                        "MACRO or struct call",
                        preceded(space0, parse_macro_or_struct_call(false, false))
                    ))(input2.clone())?;
                    (
                        input3,
                        LocatedToken::Standard {
                            token: macro_call,
                            span: before_label.clone()
                        }
                    )
                }

                Ok((input2, opcode)) => {
                    // any other token is a normal token
                    (input2, opcode)
                }

                Err(e) => return Err(e)
            };

            tokens.push(opcode);
            input2
        }
        else {
            input
        }
    };

    // Eat the additional opcodes after the label (with modifier or not, the macro or the opcode)
    let (input, additional_opcodes) = context(
        "[DBG] other tokens",
        cut(fold_many0(
            parse_single_token(false),
            || Vec::new(),
            |mut acc: Vec<_>, item| {
                acc.push(item);
                acc
            }
        ))
    )(input)?;

    let (input, comment) = if preceded(tag(":"), parse_forbidden_keyword)(input.clone()).is_err() {
        // we have not an ending keyword here

        // eat extra : as in Targhans code
        let (input, _) = opt(delimited(space0, tag(":"), space0))(input)?;

        // Eat final comment
        let (input, _) = space0(input)?;
        let before_comment = input.clone();
        let (input, comment) = opt(parse_comment)(input)?;
        let (input, _) = space0(input)?;

        // Ensure it is the end of line of file or a forbidden keyword
        let (input, _) = cut(context(
            "We expect nothing else at the end of the line",
            alt((line_ending, eof))
        ))(input)?;
        (input, comment.map(|c| c.locate(before_comment)))
    }
    else {
        (input.take_split(1).0, None) // Remove the #
    };

    for opcode in additional_opcodes {
        tokens.push(opcode);
    }
    if comment.is_some() {
        tokens.push(unsafe { comment.unwrap_unchecked() });
    }

    Ok((input, tokens))
}

/// No opcodes are expected there.
/// Initially it was supposed to manage lines with only labels, however it has been extended
/// to labels fallowed by specific commands.
/// TODO this complete piece of code MUST be removed and integrated within parse_z80_line_complete
pub fn parse_z80_line_label_only(
    input: Z80Span
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    let before_label = input.clone();

    let (input, r#let) = opt(delimited(space0, parse_directive_word("LET"), space0))(input)?;
    let _after_let = input.clone();
    let (input, label) = context("Label issue", preceded(space0, parse_label(true)))(input)?;

    #[derive(Clone, Copy)]
    enum LabelModifier {
        Equ,
        Set,
        Equal,
        SetN,
        Next
    }

    // TODO make these stuff alternatives ...
    // Manage Equ
    // BUG Equ and = are supposed to be different
    let (input, label_modifier) = opt(alt((
        map(preceded(space1, parse_directive_word("DEFL")), |_| {
            LabelModifier::Equ
        }),
        map(preceded(space1, parse_directive_word("EQU")), |_| {
            LabelModifier::Equ
        }),
        map(preceded(space1, parse_directive_word("SETN")), |_| {
            LabelModifier::SetN
        }),
        map(preceded(space1, parse_directive_word("NEXT")), |_| {
            LabelModifier::Next
        }),
        map(
            delimited(
                space1,
                parse_directive_word("SET"),
                not(tuple((space0, expr, parse_comma)))
            ),
            |_| LabelModifier::Set
        ),
        map(delimited(space0, tag("="), space0), |_| {
            LabelModifier::Equal
        })
    )))(input)?;

    // ensure let uses =
    if r#let.is_some() {
        if let Some(LabelModifier::Equal) = &label_modifier {
            // ok
        }
        else {
            return Err(cpclib_common::nom::Err::Failure(
                VerboseError::from_error_kind(before_label, ErrorKind::Char)
            ));
        }
    }

    let (input, expr_arg) = match &label_modifier {
        Some(LabelModifier::Equ | LabelModifier::Equal | LabelModifier::Set) => {
            cut(context("Value error", map(expr, |e| Some(e))))(input)?
        }
        _ => (input, None)
    };

    let (input, source_label) = match &label_modifier {
        Some(LabelModifier::Next | LabelModifier::SetN) => {
            cut(context(
                "Label expected",
                map(parse_label(false), |l| Some(l))
            ))(input)?
        }
        _ => (input, None)
    };

    // optional expression to control the displacement
    let (input, additional_arg) = match &label_modifier {
        Some(LabelModifier::Next | LabelModifier::SetN) => opt(preceded(parse_comma, expr))(input)?,
        _ => (input, None)
    };

    // opt!(char!(':')) >>

    let before_comment = input.clone();
    let (input, comment) = delimited(space0, opt(parse_comment), alt((line_ending, eof)))(input)?;

    {
        let mut tokens = Vec::new();

        // Build the needed token for the label of interest
        let token = match label_modifier {
            Some(LabelModifier::Equ) => Token::Equ(label, expr_arg.unwrap()),
            Some(LabelModifier::Equal | LabelModifier::Set) => {
                Token::Assign(label, expr_arg.unwrap())
            }
            Some(LabelModifier::SetN) => Token::SetN(label, source_label.unwrap(), additional_arg),
            Some(LabelModifier::Next) => Token::Next(label, source_label.unwrap(), additional_arg),
            None => Token::Label(label)
        };

        // add it to the list
        tokens.push(token.locate(before_label));

        if comment.is_some() {
            tokens.push(comment.unwrap().locate(before_comment));
        }

        Ok((input, tokens))
    }
}
pub fn parse_fname(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    parse_string(input)
}

/// Parser for file names in appropriate directives
pub fn parse_string(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    alt((
    preceded(tag("\""), terminated(take_until("\""), take(1usize))),
    verify(
        preceded(tag("'"), terminated(take_until("'"), take(1usize))),
        |s: &Z80Span| s.len() > 1
    ),
    )) // single quote is stricly reserved for chars now, so we accept strings with 2 chars at minimum
    (input)
}

pub fn parse_charset(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("CHARSET")(input)?;

    let (input, charset) = opt(parse_charset_string)(input)?;

    Ok((
        input,
        charset.unwrap_or_else(|| Token::Charset(CharsetFormat::Reset))
    ))
}

pub fn parse_charset_string(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    // manage the string format - TODO manage the others too
    let (input, chars) = context("Invalid string", parse_decoded_string)(input)?;
    let (input, start) = context("Missing start value", preceded(parse_comma, expr))(input)?;
    let format = CharsetFormat::CharsList(chars.chars().collect_vec(), start);

    let charset = Token::Charset(format);
    Ok((input, charset))
}

/// Parser for the include directive
pub fn parse_include(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let include_start = input.clone();
    let (input, once_fname) = preceded(
        alt((parse_directive_word("INCLUDE"), parse_word("READ"))),
        pair(
            opt(delimited(space0, parse_word("ONCE"), space0)),
            parse_fname
        )
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

    let size = include_start.len()-input.len();
    Ok((
        input,
        LocatedToken::Standard{
            token: Token::Include(
                fname.to_string(),
                namespace,
                once.is_some()
            ),
            span: include_start.take(size)
        }
        

    ))
}

/// Parse for the various binary include directives
pub fn parse_incbin(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, transformation) = alt((
        map(
            alt((tag_no_case("INCBIN"), tag_no_case("BINCLUDE"))),
            |_| BinaryTransformation::None
        ),
        map(tag_no_case("INCEXO"), |_| BinaryTransformation::Exomizer),
        map(tag_no_case("INCL48"), |_| BinaryTransformation::Lz48),
        map(tag_no_case("INCL49"), |_| BinaryTransformation::Lz49),
        map(tag_no_case("INCAPU"), |_| BinaryTransformation::Aplib)
    ))(input)?;

    let (input, fname) = preceded(space1, parse_fname)(input)?;

    let (input, offset) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, length) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, _extended_offset) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, off) = opt(preceded(
        tuple((space0, char(','), space0)),
        tag_no_case("OFF")
    ))(input)?;

    Ok((
        input,
        Token::Incbin {
            fname: fname.to_string(),
            offset,
            length,
            extended_offset: None,
            off: off.is_some(),
            transformation
        }
    ))
}

/// parse write direct in memory / converted to a bank directive
/// we do not care of the parameters for roms as we are not working in an emulator
pub fn parse_write_direct_memory(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let input_start = input.clone();

    // filter all the stuff before
    let (input, _) = tuple((
        tag_no_case("WRITE"),
        space1,
        tag_no_case("DIRECT"),
        space1,
        tag_no_case("-1"),
        parse_comma,
        tag_no_case("-1"),
        parse_comma
    ))(input)?;

    let (input, bank) = expr(input)?;

    // TODO add an additional note that
    let warning = AssemblerError::RelocatedWarning {
        warning: Box::new(AssemblerError::AssemblingError {
            msg: "Prefer BANK or PAGE directives to write direct -1, -1, XX".to_owned()
        }),
        span: input_start.clone()
    };
    input.extra.1.add_warning(warning);

    Ok((input, Token::Bank(Some(bank))))
}

/// Parse both save directive and write direct in a file
pub fn parse_save(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    #[derive(PartialEq)]
    enum SaveKind {
        Save,
        WriteDirect
    }

    let (input, (save_kind, filename)) = pair(
        alt((
            map(parse_directive_word("SAVE"), |_| SaveKind::Save),
            map(tuple((parse_word("WRITE"), parse_word("DIRECT"))), |_| {
                SaveKind::WriteDirect
            })
        )),
        parse_fname
    )(input)?;

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
    ))
}

/// Parse  UNDEF directive.
pub fn parse_undef(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, label) =
        preceded(tuple((tag_no_case("UNDEF"), space1)), parse_label(false))(input)?;

    Ok((input, Token::Undef(label)))
}

/// Parse return from a function
pub fn parse_return(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, expr) = preceded(parse_directive_word("RETURN"), expr)(input)?;

    Ok((input, Token::Return(expr)))
}

pub fn parse_section(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("SECTION")(input)?;
    let (input, name) = preceded(space0, parse_label(false))(input)?;

    Ok((input, Token::Section(name)))
}

pub fn parse_range(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = alt((
        parse_directive_word("DEFSECTION"),
        parse_directive_word("RANGE")
    ))(input)?;

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

    Ok((input, Token::Range(label.into(), start, stop)))
}

pub fn parse_assign(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, (label, value)) = pair(
        terminated(parse_label(false), delimited(space0, tag("="), space0)),
        expr
    )(input)?;

    Ok((input, Token::Assign(label, value)))
}

/// Parse the opcodes. TODO rename as parse_opcode ...
pub fn parse_token(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let parsing_state = input.context().state.clone();

    verify(
        alt((
            parse_ex_af,
            parse_ex_hl_de,
            parse_ex_mem_sp,
            parse_logical_operator,
            parse_add_or_adc,
            parse_cp,
            parse_djnz,
            parse_ld,
            parse_inc_dec,
            parse_out,
            parse_in,
            parse_call_jp_or_jr,
            parse_opcode_no_arg,
            parse_push_n_pop,
            parse_res_set_bit,
            parse_shifts_and_rotations,
            parse_sub,
            parse_sbc,
            parse_ret,
            parse_rst,
            parse_im
        )),
        move |t| t.is_accepted(&parsing_state)
    )(input.clone())
    .map(|(i, r)| (i, r.locate(input)))
}

/// Parse ex af, af' instruction
pub fn parse_ex_af(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        alt((
            value(
                (),
                tuple((
                    parse_word("EX"),
                    parse_register_af,
                    parse_comma,
                    parse_word("AF'")
                ))
            ),
            value((), parse_word("EXA"))
        )),
        |_| Token::new_opcode(Mnemonic::ExAf, None, None)
    )(input)
}

/// Parse ex hl, de instruction
pub fn parse_ex_hl_de(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        alt((
            value(
                (),
                tuple((
                    tag_no_case("EX"),
                    space1,
                    parse_register_hl,
                    parse_comma,
                    parse_register_de
                ))
            ),
            value(
                (),
                tuple((
                    tag_no_case("EX"),
                    space1,
                    parse_register_de,
                    parse_comma,
                    parse_register_hl
                ))
            ),
            value((), parse_word("EXD"))
        )),
        |_| Token::new_opcode(Mnemonic::ExHlDe, None, None)
    )(input)
}

/// Parse ex (sp), hl
pub fn parse_ex_mem_sp(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, destination) = tuple((
        tag_no_case("EX"),
        space1,
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
        Token::new_opcode(Mnemonic::ExMemSp, Some(destination.8), None)
    ))
}

pub fn parse_directive1(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let dir_start = input.clone();
    alt((
        context("[DBG] include", parse_include),
        map(
            alt((
                context("[DBG] assert", parse_assert),
                context("[DBG] bankset", parse_bankset),
                context("[DBG] bank", parse_bank),
                context("[DBG] charset", parse_charset),
                context("[DBG] align", parse_align),
                context("[DBG] breakpoint", parse_breakpoint),
                context("[DBG] buildsna", parse_buildsna),
                context("[DBG] org", parse_org),
                context("[DBG] defs", parse_defs),
                context("[DBG] nop", parse_nop),
                context("[DBG] export", parse_export),
                context("[DBG] incbin", parse_incbin),
                context("[DBG] limit", parse_limit),
                context("[DBG] db", parse_db_or_dw_or_str),
                context("[DBG] print", parse_print),
                context("[DBG] fail", parse_fail),
                context("[DBG] protext", parse_protect),
                context("[DBG] run", parse_run),
                context("[DBG] snaset", parse_snaset),
                map(preceded(space0, parse_directive_word("PAUSE")), |_| {
                    Token::Pause
                })
            )),
            move |t| t.locate(dir_start.clone())
        )
    ))(input.clone())
}

pub fn parse_directive2(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let dir_start = input.clone();

    let (input2, directive) = alt((
        context("[DBG] write direct memory", parse_write_direct_memory),
        context("[DBG] save", parse_save),
        context("[DBG] ticker", parse_stable_ticker),
        context("[DBG] waitnops", parse_waitnops),
        context("[DBG] struct", parse_struct),
        context("[DBG] undef", parse_undef),
        context("[DBG] return", parse_return),
        context("[DBG] noargs", parse_noarg_directive),
        context("[DBG] assign", parse_assign),
        context("[DBG] range", parse_range),
        context("[DBG] section", parse_section),
        context("[DBG] snainit", parse_snainit),
        context(
            "[DBG] macro or struct call",
            parse_macro_or_struct_call(true, false)
        )
    ))(input.clone())?;

    // XXX Do we keep that
    let directive = directive.locate(Z80Span(unsafe {
        LocatedSpan::new_from_raw_offset(
            dir_start.location_offset(),
            dir_start.location_line(),
            &dir_start[..dir_start.len() - input2.len()],
            dir_start.extra.clone()
        )
    }));
    Ok((input2, directive))
}

/// Parse any directive
pub fn parse_directive(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let parsing_state = input.context().state.clone();
    verify(alt((parse_directive1, parse_directive2)), move |d| {
        d.is_accepted(&parsing_state)
    })(input.clone())
}

/// Parse directives with no arguments
pub fn parse_noarg_directive(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    alt((
        map(tag_no_case("list"), |_| Token::List),
        map(tag_no_case("nolist"), |_| Token::NoList)
    ))(input)
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
pub fn parse_conditional(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
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
                map(delimited(space0, parse_comment, line_ending), |_| "".into()),
                line_ending,
                tag(":")
            ))
        ))(input)?;

        let code_input = input.clone();
        let (input, code) = cut(context(
            "Condition: syntax error in code condition",
            inner_code
        ))(input)?;

        let code = LocatedListing::try_from(code)
            .unwrap_or_else(|_| LocatedListing::new_empty_span(code_input));

        if let Some(condition) = condition {
            conditions.push((condition, code));

            let (input, r#else) = opt(preceded(
                many0(alt((space1, line_ending, tag(":")))),
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
                map(delimited(space0, parse_comment, line_ending), |_| "".into())
            ))),
            cut(preceded(space0, parse_directive_word("ENDIF")))
        ))
    )(input_loop)?;

    Ok((input, LocatedToken::If(conditions, else_clause, if_start)))
}

/// Read the condition part in the parse_conditional macro
fn parse_conditional_condition(
    code: KindOfConditional
) -> impl Fn(Z80Span) -> IResult<Z80Span, TestKind, VerboseError<Z80Span>> {
    move |input: Z80Span| -> IResult<Z80Span, TestKind, VerboseError<Z80Span>> {
        match &code {
            KindOfConditional::If => map(expr, |e| TestKind::True(e))(input),

            KindOfConditional::IfNot => map(expr, |e| TestKind::False(e))(input),

            KindOfConditional::IfDef => {
                map(preceded(space0, parse_label(false)), |l| {
                    TestKind::LabelExists(l)
                })(input)
            }

            KindOfConditional::IfNdef => {
                map(parse_label(false), |l| TestKind::LabelDoesNotExist(l))(input)
            }

            KindOfConditional::IfUsed => map(parse_label(false), |l| TestKind::LabelUsed(l))(input),

            KindOfConditional::IfNused => {
                map(parse_label(false), |l| TestKind::LabelNused(l))(input)
            }

            _ => unreachable!()
        }
    }
}

/// Parse a breakpint instruction
pub fn parse_breakpoint(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(
            delimited(space0, parse_directive_word("BREAKPOINT"), space0),
            opt(expr)
        ),
        |exp| Token::Breakpoint(exp)
    )(input)
}

pub fn parse_bankset(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("bankset")(input)?;
    let (input, count) = expr(input)?;

    Ok((input, Token::Bankset(count)))
}

pub fn parse_buildsna(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    terminated(
        map(
            preceded(
                parse_directive_word("BUILDSNA"),
                cut(opt(alt((tag_no_case("V2"), tag_no_case("V3")))))
            ),
            |v| {
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

pub fn parse_run(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, exp) = preceded(parse_directive_word("RUN"), expr)(input)?;
    let (input, ga) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;

    Ok((input, Token::Run(exp, ga)))
}

pub fn parse_limit(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, exp) = preceded(parse_directive_word("LIMIT"), expr)(input)?;

    Ok((input, Token::Limit(exp)))
}

pub fn parse_waitnops(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, exp) = preceded(parse_directive_word("WAITNOPS"), expr)(input)?;

    Ok((input, Token::WaitNops(exp)))
}

/// Parse tickin directives
pub fn parse_stable_ticker(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    alt((parse_stable_ticker_start, parse_stable_ticker_stop))(input)
}

/// Parse begining of ticker
pub fn parse_stable_ticker_start(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(
            tuple((
                opt(tag_no_case("stable")),
                tag_no_case("ticker"),
                space1,
                tag_no_case("start"),
                space1
            )),
            parse_label(false)
        ),
        |name| Token::StableTicker(StableTickerAction::Start(name))
    )(input)
}

/// Parse end of ticker
pub fn parse_stable_ticker_stop(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        tuple((
            opt(tag_no_case("stable")),
            tag_no_case("ticker"),
            space1,
            tag_no_case("stop")
        )),
        |_| Token::StableTicker(StableTickerAction::Stop)
    )(input)
}

pub fn parse_bank(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("bank")(input)?;
    let (input, count) = opt(expr)(input)?;

    Ok((input, Token::Bank(count)))
}

/// Parse fake and real LD instructions
pub fn parse_ld(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    context(
        "[DBG] ld",
        alt((
            context("[DBG] fake ld", parse_ld_fake),
            context("[DBG] normal ld", parse_ld_normal)
        ))
    )(input)
}

/// Parse artifical LD instruction (would be replaced by several real instructions)
pub fn parse_ld_fake(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let input_start = input.clone();
    let (input, _) = tuple((tag_no_case("LD"), space1))(input)?;

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

    let warning = AssemblerError::RelocatedWarning {
        warning: Box::new(AssemblerError::AssemblingError {
            msg: "Fake instruction assembled using several opcodes".to_owned()
        }),
        span: input_start.clone()
    };
    input.extra.1.add_warning(warning);

    Ok((input, Token::new_opcode(Mnemonic::Ld, Some(dst), Some(src))))
}

/// Parse the valids LD versions
pub fn parse_ld_normal(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = context("[DBG] ...", tuple((space0, parse_word("LD"), space0)))(input)?;

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

    Ok((input, Token::new_opcode(Mnemonic::Ld, Some(dst), Some(src))))
}

/// Parse the source of LD depending on its destination
fn parse_ld_normal_src(
    dst: &DataAccess
) -> impl Fn(Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> + '_ {
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
                VerboseError::from_error_kind(input, ErrorKind::Alt)
            ))
        }
    }
}

/// Parse RES, SET and BIT instructions
pub fn parse_res_set_bit(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, res_or_set) = alt((
        map(tag_no_case("RES"), |_| Mnemonic::Res),
        map(tag_no_case("BIT"), |_| Mnemonic::Bit),
        map(tag_no_case("SET"), |_| Mnemonic::Set)
    ))(input)?;

    let (input, bit) = cut(context(
        "Wrong bit definition",
        preceded(space1, parse_expr)
    ))(input)?;

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

/// Parse CP tokens
pub fn parse_cp(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(
            parse_word("CP"),
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr
            ))
        ),
        |operand| Token::new_opcode(Mnemonic::Cp, Some(operand), None)
    )(input)
}

pub fn parse_export(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, code) = alt((
        map(parse_directive_word("EXPORT"), |_| 0),
        map(parse_directive_word("NOEXPORT"), |_| 1)
    ))(input)?;

    let (input, labels) = cut(context(
        "Wrong parameters",
        separated_list0(parse_comma, parse_label(false))
    ))(input)?;

    if code == 0 {
        Ok((input, Token::Export(labels)))
    }
    else {
        Ok((input, Token::NoExport(labels)))
    }
}
/// Parse DB DW directives
pub fn parse_db_or_dw_or_str(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, code) = alt((
        map(
            alt((
                parse_directive_word("BYTE"),
                parse_directive_word("TEXT"),
                parse_directive_word("DB"),
                parse_directive_word("DEFB"),
                parse_directive_word("DM"),
                parse_directive_word("DEFM")
            )),
            |_| 0
        ),
        map(
            alt((
                parse_directive_word("WORD"),
                parse_directive_word("DW"),
                parse_directive_word("DEFW")
            )),
            |_| 1
        ),
        map(parse_directive_word("STR"), |_| 2)
    ))(input)?;

    let (input, expr) = expr_list(input)?;

    Ok((
        input,
        if code == 0 {
            Token::Defb(expr)
        }
        else if code == 1 {
            Token::Defw(expr)
        }
        else
        // if code == 2
        {
            Token::Str(expr)
        }
    ))
}

// Fail if we do not read a forbidden keyword
pub fn parse_forbidden_keyword(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    let (input, _) = space0(input)?;
    let (input, name) = context(
        "Unable to read directive name",
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")
    )(input)?;

    let mut end_directive_iter = if input.context().dotted_directive {
        DOTTED_END_DIRECTIVE.iter()
    }
    else {
        END_DIRECTIVE.iter()
    };
    if !end_directive_iter
        .find(|&&a| a == name.to_uppercase())
        .is_some()
    {
        return Err(Err::Error(VerboseError::from_error_kind(
            name,
            ErrorKind::AlphaNumeric
        )));
    }

    let (input, _) = space0(input)?;

    Ok((input, name))
}
pub fn parse_macro_arg(input: Z80Span) -> IResult<Z80Span, MacroParam, VerboseError<Z80Span>> {
    alt((
        map(
            delimited(
                tuple((space0, char('['))),
                separated_list0(char(','), parse_macro_arg),
                tuple((char(']'), space0))
            ),
            |l| {
                MacroParam::List(
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
                    map(recognize(expr), |s| s.to_string()), /* TODO handle evaluation or transposition */
                    map(string_between_quotes, |s| s.to_string()),
                    map(many0(none_of(" ,\r\n\t][;")), |s| {
                        s.iter().collect::<String>().trim().to_owned()
                    })
                )), // TODO find a way to give arguments with space
                alt((space0, eof))
            ),
            |s| MacroParam::Single(s)
        )
    ))(input)
}

/// Manage the call of a macro.
/// When ambiguou may return a label
pub fn parse_macro_or_struct_call(
    can_return_label: bool,
    for_struct: bool
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    move |input| {
        // BUG: added because of parsing issues. Need to find why and remove ot
        let (input_label, _) = space0(input)?;
        let (input, name) = parse_macro_name(input_label.clone())?;

        // Check if the macro name is allowed
        if impossible_names(input.context().dotted_directive).any(|&a| a == name.to_uppercase()) {
            Err(Err::Failure(cpclib_common::nom::error::VerboseError::<
                Z80Span
            >::add_context(
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
            )))
        }
        else {
            if can_return_label && pair(space0, opt(parse_comment))(input.clone()).is_ok() {
                input.extra.1.add_warning(AssemblerError::RelocatedWarning{
                warning: Box::new(AssemblerError::AssemblingError{
                    msg: format!("Ambiguous code. Use (void) for macro with no args, (default) for struct with default parameters; avoid labels that do not start at beginning of a line. {} is considered to be a label, not a macro.", name)
                }),
                span: input.clone()
            });
                return Ok((input, Token::Label(name)));
            }

            let (input, _) = space0(input)?;

            let (input, args) = if alt((eof, tag("\n"), tag(":")))(input.clone()).is_ok() {
                // panic!("no arguments at all provided")
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
                                    map(space1, |_| MacroParam::Single("".to_owned()))
                                ))
                            )
                        ))
                    ))
                ))(input)?
            };

            if args.len() == 1 && args.first().unwrap().is_empty() {
                panic!();
            }

            // avoid ambiguate code such as label nop
            if args.len() == 1 {
                let arg = args[0].to_string().to_lowercase();
                if arg == "nop" || parse_opcode_no_arg(Z80Span::from(arg)).is_ok() {
                    return Err(Err::Failure(cpclib_common::nom::error::VerboseError::<
                        Z80Span
                    >::add_context(
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
                    )));
                }
            }

            Ok((input, Token::MacroCall(name, args)))
        }
    }
}

fn parse_directive_word(
    name: &'static str
) -> impl Fn(Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> + 'static {
    move |input: Z80Span| {
        if input.context().dotted_directive {
            preceded(tag("."), parse_word(name))(input)
        }
        else {
            parse_word(name)(input)
        }
    }
}

fn parse_word(
    name: &'static str
) -> impl Fn(Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> + 'static {
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
pub fn parse_djnz(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(terminated(parse_word("DJNZ"), opt(parse_comma)), parse_expr),
        |expr| Token::new_opcode(Mnemonic::Djnz, Some(expr), None)
    )(input)
}

/// ...
pub fn expr_list(input: Z80Span) -> IResult<Z80Span, Vec<Expr>, VerboseError<Z80Span>> {
    separated_list1(
        tuple((tag(","), space0)),
        cut(context("Error in expression", alt((expr, string_expr))))
    )(input)
}

/// ...
pub fn parse_assert(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = context("assert", preceded(space0, parse_directive_word("ASSERT")))(input)?;

    let (input, expr) = cut(context("ASSERT: expression error", expr))(input)?;

    let (input, exps) = cut(context(
        "ASSERT: comment error",
        opt(preceded(parse_comma, parse_print_inner))
    ))(input)?;

    Ok((input, Token::Assert(expr, exps)))
}

/// ...
pub fn parse_align(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, boundary) = preceded(parse_directive_word("ALIGN"), expr)(input)?;
    let (input, fill) = opt(preceded(parse_comma, expr))(input)?;

    Ok((input, Token::Align(boundary, fill)))
}

pub fn parse_print_inner(
    input: Z80Span
) -> IResult<Z80Span, Vec<FormattedExpr>, VerboseError<Z80Span>> {
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
pub fn parse_print(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(parse_directive_word("PRINT"), cut(parse_print_inner)),
        |exps| Token::Print(exps)
    )(input)
}

pub fn parse_fail(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(parse_directive_word("FAIL"), cut(parse_print_inner)),
        |exps| Token::Fail(exps)
    )(input)
}

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
fn formatted_expr(input: Z80Span) -> IResult<Z80Span, FormattedExpr, VerboseError<Z80Span>> {
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
fn my_space0(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    recognize(opt(my_space1))(input)
}

/// Handle \ in end of line
fn my_space1(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    alt((
        recognize(eof),
        recognize(tuple((
            space0,
            tag("\\"), // do we keep it ?
            opt(pair(space0, parse_comment)),
            line_ending,
            space0
        ))),
        recognize(space1)
    ))(input)
}

fn parse_comma(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    delimited(my_space0, tag(","), my_space0)(input)
}

/// ...
pub fn parse_protect(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, start) = preceded(parse_directive_word("PROTECT"), expr)(input)?;

    let (input, end) = preceded(parse_comma, expr)(input)?;

    Ok((input, Token::Protect(start, end)))
}

/// ...
pub fn parse_logical_operator(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, operator) = alt((
        map(parse_word("AND"), |_| Mnemonic::And),
        map(parse_word("Or"), |_| Mnemonic::Or),
        map(parse_word("Xor"), |_| Mnemonic::Xor)
    ))(input)?;

    let (input, operand) = cut(context(
        "logical operand",
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr
        ))
    ))(input)?;

    Ok((input, Token::new_opcode(operator, Some(operand), None)))
}

/// ...
pub fn parse_add_or_adc(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    alt((parse_add_or_adc_complete, parse_add_or_adc_shorten))(input)
}

/// Substraction with A register
pub fn parse_sub(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = tag_no_case("SUB")(input)?;
    let (input, _) = space1(input)?;
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
pub fn parse_sbc(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = tag_no_case("SBC")(input)?;
    let (input, _) = space1(input)?;

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
pub fn parse_add_or_adc_complete(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, add_or_adc) = alt((
        map(tag_no_case("ADC"), |_| Mnemonic::Adc),
        map(tag_no_case("ADD"), |_| Mnemonic::Add)
    ))(input)?;

    let (input, _) = space1(input)?;

    let (input, first) = alt((
        map(parse_register_a, |_| DataAccess::Register8(Register8::A)),
        map(parse_register_hl, |_| {
            DataAccess::Register16(Register16::Hl)
        }),
        parse_indexregister16
    ))(input)?;

    let (input, _) = tuple((space0, tag(","), space0))(input)?;

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
            VerboseError::from_error_kind(input, ErrorKind::Alt)
        ));
    }?;

    Ok((
        input,
        Token::new_opcode(add_or_adc, Some(first), Some(second))
    ))
}

/// TODO Find a way to not duplicate code with complete version
pub fn parse_add_or_adc_shorten(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, add_or_adc) = alt((
        map(parse_word("ADC"), |_| Mnemonic::Adc),
        map(parse_word("ADD"), |_| Mnemonic::Add)
    ))(input)?;

    let (input, second) = alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr
    ))(input)?;

    Ok((
        input,
        Token::new_opcode(
            add_or_adc,
            Some(DataAccess::Register8(Register8::A)),
            Some(second)
        )
    ))
}

/// ...
pub fn parse_push_n_pop(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, push_or_pop) = alt((
        map(parse_word("PUSH"), |_| Mnemonic::Push),
        map(parse_word("POP"), |_| Mnemonic::Pop)
    ))(input)?;

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

/// ...
pub fn parse_ret(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(preceded(parse_word("RET"), opt(parse_flag_test)), |cond| {
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
pub fn parse_inc_dec(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, inc_or_dec) = alt((
        map(parse_word("INC"), |_| Mnemonic::Inc),
        map(parse_word("DEC"), |_| Mnemonic::Dec)
    ))(input)?;

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

/// TODO manage other out formats
pub fn parse_out(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_word("OUT")(input)?;

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
pub fn parse_in(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_word("IN")(input)?;
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
pub fn parse_rst(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_word("RST")(input)?;
    let (input, val) = parse_expr(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Rst, Some(val), None)))
}

/// Parse the IM instruction
pub fn parse_im(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_word("IM")(input)?;
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
pub fn parse_shifts_and_rotations(
    input: Z80Span
) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, oper) = alt((
        map(parse_word("RLC"), |_| Mnemonic::Rlc),
        map(parse_word("RRC"), |_| Mnemonic::Rrc),
        map(parse_word("RL"), |_| Mnemonic::Rl),
        map(parse_word("RR"), |_| Mnemonic::Rr),
        map(parse_word("SLA"), |_| Mnemonic::Sla),
        map(parse_word("SRA"), |_| Mnemonic::Sra),
        map(parse_word("SRL"), |_| Mnemonic::Srl),
        map(parse_word("SL1"), |_| Mnemonic::Sl1),
        map(parse_word("SLL"), |_| Mnemonic::Sl1)
    ))(input)?;

    let (input, arg) = alt((
        parse_register8,
        parse_hl_address,
        parse_indexregister_with_index
    ))(input)?;

    // hidden opcodes
    let (input, arg2) = opt(preceded(parse_comma, parse_register8))(input)?;

    Ok((input, Token::new_opcode(oper, Some(arg), arg2)))
}

/// TODO reduce the flag space for jr"],
pub fn parse_call_jp_or_jr(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, call_jp_or_jr) = alt((
        map(parse_word("JP"), |_| Mnemonic::Jp),
        map(parse_word("JR"), |_| Mnemonic::Jr),
        map(parse_word("CALL"), |_| Mnemonic::Call)
    ))(input)?;

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

/// ...
pub fn parse_flag_test(input: Z80Span) -> IResult<Z80Span, FlagTest, VerboseError<Z80Span>> {
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
pub fn parse_register16(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    alt((
        parse_register_hl,
        parse_register_bc,
        parse_register_de,
        parse_register_af
    ))(input)
}

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
pub fn parse_register8(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_register_i(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::SpecialRegisterI,
        tuple((tag_no_case("I"), not(alphanumeric1)))
    )(input)
}

/// Parse register r
pub fn parse_register_r(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::SpecialRegisterR,
        tuple((tag_no_case("R"), not(alphanumeric1)))
    )(input)
}

macro_rules! parse_any_register8 {
    ($name: ident, $char:expr, $reg:expr) => {
        /// Parse register $char
        pub fn $name(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
fn register16_parser(
    representation: &'static str,
    register: Register16
) -> impl for<'src, 'ctx> Fn(Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
        pub fn $name(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_register_ix(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::IndexRegister16(IndexRegister16::Ix),
        tuple((tag_no_case("IX"), not(alphanumeric1)))
    )(input)
}

/// Parse the IY register
pub fn parse_register_iy(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
            pub fn [<parse_register_ $reg:lower>] (input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_indexregister8(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    alt((
        parse_register_ixh,
        parse_register_iyh,
        parse_register_ixl,
        parse_register_iyl
    ))(input)
}

/// Parse a 16 bits indexed register
pub fn parse_indexregister16(
    input: Z80Span
) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    let (input, reg) = preceded(tuple((tag("("), space0)), parse_indexregister16)(input)?;

    let (input, op) = preceded(
        space0,
        alt((value(Oper::Add, tag("+")), value(Oper::Sub, tag("-"))))
    )(input)?;

    let (input, expr) = terminated(expr, tuple((space0, tag(")"))))(input)?;

    Ok((
        input,
        DataAccess::IndexRegister16WithIndex(
            reg.get_indexregister16().unwrap(),
            match op {
                Oper::Add => expr,
                Oper::Sub => expr.neg(),
                _ => unreachable!()
            }
        )
    ))
}

/// Parse (C) used in in/out
pub fn parse_portc(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::PortC,
        tuple((tag("("), space0, parse_register_c, space0, tag(")")))
    )(input)
}

/// Parse (nn) used in in/out
pub fn parse_portnn(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    map(
        delimited(tag("("), expr, preceded(space0, tag(")"))),
        |address| DataAccess::PortN(address)
    )(input)
}

/// Parse an address access `(expression)`
pub fn parse_address(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_reg_address(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_hl_address(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_indexregister_address(
    input: Z80Span
) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
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
pub fn parse_expr(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    let (input, expr) = expr(input)?;
    Ok((input, DataAccess::Expression(expr)))
}

/// Parse standard org directive
pub fn parse_org(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("ORG")(input)?;

    let (input, val1) = cut(context("Invalid argument", expr))(input)?;
    let (input, val2) = opt(preceded(parse_comma, expr))(input)?;

    Ok((input, Token::Org(val1, val2)))
}

/// Parse defs instruction. TODO add optional parameters
pub fn parse_defs(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = alt((
        parse_directive_word("FILL"),
        parse_directive_word("DS"),
        parse_directive_word("DEFS"),
        parse_directive_word("RMEM")
    ))(input)?;

    let (input, val) = separated_list1(
        parse_comma,
        cut(context(
            "Wrong argument",
            tuple((expr, opt(preceded(parse_comma, expr))))
        ))
    )(input)?;

    Ok((input, Token::Defs(val)))
}

pub fn parse_nop(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_word("NOP")(input)?;

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
pub fn parse_opcode_no_arg(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    alt((
        parse_opcode_no_arg1,
        parse_opcode_no_arg2,
        parse_opcode_no_arg3
    ))(input)
}

fn parse_opcode_no_arg1(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, mnemonic) = alt((
        map(parse_word("DI"), |_| Mnemonic::Di),
        map(parse_word("CCF"), |_| Mnemonic::Ccf),
        map(parse_word("EI"), |_| Mnemonic::Ei),
        map(parse_word("EXX"), |_| Mnemonic::Exx),
        map(parse_word("HALT"), |_| Mnemonic::Halt),
        map(parse_word("LDIR"), |_| Mnemonic::Ldir),
        map(parse_word("LDDR"), |_| Mnemonic::Lddr),
        map(parse_word("LDI"), |_| Mnemonic::Ldi),
        map(parse_word("LDD"), |_| Mnemonic::Ldd),
        map(parse_word("NOPS2"), |_| Mnemonic::Nop2),
        map(parse_word("OUTD"), |_| Mnemonic::Outd),
        map(parse_word("OUTI"), |_| Mnemonic::Outi)
    ))(input)?;

    Ok((input, Token::new_opcode(mnemonic, None, None)))
}

fn parse_opcode_no_arg2(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, mnemonic) = alt((
        map(parse_word("CPD"), |_| Mnemonic::Cpd),
        map(parse_word("CPDR"), |_| Mnemonic::Cpdr),
        map(parse_word("CPI"), |_| Mnemonic::Cpi),
        map(parse_word("CPIR"), |_| Mnemonic::Cpir),
        map(parse_word("CPL"), |_| Mnemonic::Cpl),
        map(parse_word("IND"), |_| Mnemonic::Ind),
        map(parse_word("INDR"), |_| Mnemonic::Indr),
        map(parse_word("INI"), |_| Mnemonic::Ini),
        map(parse_word("INIR"), |_| Mnemonic::Inir),
        map(parse_word("RETI"), |_| Mnemonic::Reti),
        map(parse_word("RETN"), |_| Mnemonic::Retn),
        map(parse_word("RLA"), |_| Mnemonic::Rla),
        map(parse_word("RLCA"), |_| Mnemonic::Rlca),
        map(parse_word("RRA"), |_| Mnemonic::Rra),
        map(parse_word("RRCA"), |_| Mnemonic::Rrca),
        map(parse_word("SCF"), |_| Mnemonic::Scf)
    ))(input)?;

    Ok((input, Token::new_opcode(mnemonic, None, None)))
}

fn parse_opcode_no_arg3(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, mnemonic) = alt((
        map(parse_word("DAA"), |_| Mnemonic::Daa),
        map(parse_word("NEG"), |_| Mnemonic::Neg),
        map(alt((parse_word("OUTDR"), parse_word("OTDR"))), |_| {
            Mnemonic::Otdr
        }),
        map(alt((parse_word("OUTIR"), parse_word("OTIR"))), |_| {
            Mnemonic::Otir
        }),
        map(parse_word("RLD"), |_| Mnemonic::Rld),
        map(parse_word("RRD"), |_| Mnemonic::Rrd)
    ))(input)?;

    Ok((input, Token::new_opcode(mnemonic, None, None)))
}

fn parse_snainit(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = alt((
        parse_directive_word("SNAPINIT"),
        parse_directive_word("SNAINIT")
    ))(input)?;
    let (input, fname) = parse_fname(input)?;

    Ok((input, Token::SnaInit(fname.to_string())))
}

fn parse_struct(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("STRUCT")(input)?;
    let (input, name) = cut(parse_label(false))(input)?;

    let (input, fields) = cut(context(
        "STRUCT: error in inner content",
        many1(delimited(
            many0(alt((
                space1,
                recognize(parse_comment),
                line_ending,
                tag(":")
            ))),
            pair(
                context(
                    "STRUCT: label error",
                    verify(terminated(parse_label(false), space1), |label: &str| {
                        label.to_ascii_lowercase() != "endstruct"
                    })
                ),
                cut(alt((
                    parse_db_or_dw_or_str,
                    parse_macro_or_struct_call(false, true)
                )))
            ),
            many0(alt((
                space1,
                recognize(parse_comment),
                line_ending,
                tag(":")
            )))
        ))
    ))(input)?;

    let (input, _) = cut(preceded(space0, parse_directive_word("ENDSTRUCT")))(input)?;

    Ok((input, Token::Struct(name.into(), fields)))
}

fn parse_snaset(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_directive_word("SNASET")(input)?;

    let (input, flagname) = cut(context(SNASET_WRONG_LABEL, parse_label(false)))(input)?;
    let (input, _) = context(SNASET_MISSING_COMMA, cut(parse_comma))(input)?;

    let (input, values) = cut(separated_list1(
        delimited(space0, parse_comma, space0),
        parse_flag_value_inner
    ))(input)?;

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
        cpclib_common::nom::Err::Error(VerboseError::from_error_kind(
            input.clone(),
            ErrorKind::AlphaNumeric
        ))
    })?;
    Ok((input, Token::SnaSet(flag, value)))
}

/// Parse a comment that start by `;` and ends at the end of the line.
pub fn parse_comment(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(alt((tag(";"), tag("//"))), take_till(|ch| ch == '\n')),
        |string: Z80Span| Token::Comment(string.to_string())
    )(input)
}

/// Usefull later for db
pub fn string_between_quotes(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    delimited(char('\"'), is_not("\""), char('\"'))(input)
}

/// TODO
pub fn string_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(string_between_quotes, |string| {
        Expr::String(SmolStr::from(string.to_string()))
    })(input)
}

pub fn char_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(
        alt((
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
        )),
        |c| Expr::Char(c)
    )(input)
}

/// Parse a label(label: S)
/// TODO reimplement to never build a string
pub fn parse_label(
    doubledots: bool
) -> impl Fn(Z80Span) -> IResult<Z80Span, SmolStr, VerboseError<Z80Span>> {
    move |input: Z80Span| {
        // Get the label

        let (input, prefix) = opt(tag("::"))(input)?;
        let has_prefix = prefix.is_some();
        let (input, first) =
            one_of("@abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ._{}")(input)?;
        let (input, middle) = opt(is_a(
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789__.{}"
        ))(input)?;

        // fail to parse a label when it is 100% sure it corresponds to  a macro call
        let (input, macro_arg) = opt(preceded(space1, tag_no_case("(void)".into())))(input)?;
        if macro_arg.is_some() {
            return Err(cpclib_common::nom::Err::Error(error_position!(
                input,
                ErrorKind::OneOf
            )));
        }

        if (middle.is_none() && (first == '@' || first == '.' || has_prefix))
            || (has_prefix && (first == '@' || first == '.'))
        {
            return Err(cpclib_common::nom::Err::Error(error_position!(
                input,
                ErrorKind::OneOf
            )));
        }

        let input = if doubledots {
            let (input, _) = opt(tag_no_case(":"))(input)?;
            input
        }
        else {
            input
        };

        let label = format!(
            "{}{}{}",
            if has_prefix { "::" } else { "" },
            first,
            middle
                .map(|v| v.iter_elements().collect::<String>())
                .unwrap_or_default()
        );

        if impossible_names(input.context().dotted_directive)
            .any(|val| val == &label.to_uppercase())
        {
            Err(cpclib_common::nom::Err::Error(error_position!(
                input,
                ErrorKind::OneOf
            )))
        }
        else {
            Ok((input, label.into()))
        }
    }
}

fn impossible_names(dotted_directive: bool) -> impl Iterator<Item = &'static &'static str> {
    if dotted_directive {
        REGISTERS
            .into_iter()
            .chain(INSTRUCTIONS.into_iter())
            .chain(DOTTED_STAND_ALONE_DIRECTIVE.iter())
            .chain(DOTTED_START_DIRECTIVE.iter())
            .chain(DOTTED_END_DIRECTIVE.iter())
    }
    else {
        REGISTERS
            .into_iter()
            .chain(INSTRUCTIONS.into_iter())
            .chain(STAND_ALONE_DIRECTIVE.into_iter())
            .chain(START_DIRECTIVE.into_iter())
            .chain(END_DIRECTIVE.into_iter())
    }
}

pub fn parse_end_directive(input: Z80Span) -> IResult<Z80Span, String, VerboseError<Z80Span>> {
    let (input, dot) = if input.context().dotted_directive {
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

    let mut end_directive_iter = if input.context().dotted_directive {
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

pub fn parse_macro_name(input: Z80Span) -> IResult<Z80Span, SmolStr, VerboseError<Z80Span>> {
    let (input, first) = one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_")(input)?;
    let (input, name) =
        is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")(input)?;
    let first = [first];
    let keyword = chain!(first.iter().cloned(), name.iter_elements()).collect::<String>();
    let KEYWORD = keyword.to_ascii_uppercase();

    if impossible_names(input.context().dotted_directive).any(|&val| val == &KEYWORD) {
        Err(cpclib_common::nom::Err::Error(error_position!(
            input,
            ErrorKind::OneOf
        )))
    }
    else {
        Ok((input, keyword.into()))
    }
}

pub fn prefixed_label_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, prefix) = alt((
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}")),
        value(LabelPrefix::Pageset, tag_no_case("{pageset}"))
    ))(input)?;
    let (input, label) = preceded(
        space0,
        alt((
            parse_label(false),
            map(tag_no_case("$"), |_| SmolStr::from("$")),
            map(tag_no_case("$$"), |_| SmolStr::from("$$"))
        ))
    )(input)?;

    Ok((input, Expr::PrefixedLabel(prefix, label)))
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
pub fn parse_value(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let extra = input.extra.clone();
    let (input, val) =
        alt((hex_number, dec_number, bin_number))(input.clone().into()).map_err(|_op| {
            cpclib_common::nom::Err::Error(VerboseError::from_error_kind(input, ErrorKind::Verify))
        })?;

    // rebuild the rightly typed span
    let input = Z80Span::from_standard_span(input, extra);
    Ok((input, Expr::Value(val as i32)))
}

/// Parse a repetition counter
pub fn parse_counter(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(
        delimited(
            tag("{".into()),
            parse_label(false), // BUG will accept too many cases
            pair(tag("}".into()), not(alphanumeric1))
        ),
        |l| Expr::Label(format!("{{{}}}", l).into())
    )(input)
}

/// Read a parenthesed expression
pub fn parens(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    delimited(
        delimited(my_space0, tag("("), my_space0),
        map(map(expr, Box::new), Expr::Paren),
        delimited(my_space0, tag(")"), space0)
    )(input)
}

pub fn parse_expr_bracketed_list(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(
        delimited(
            pair(tag("["), my_space0),
            separated_list0(parse_comma, expr),
            pair(my_space0, tag("]"))
        ),
        |l| Expr::List(l)
    )(input)
}

pub fn parse_bool_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    alt((
        map(parse_word("true"), |_| Expr::Bool(true)),
        map(parse_word("false"), |_| Expr::Bool(false))
    ))(input)
}

// TODO rewire with https://docs.rs/nom/7.1.0/nom/bytes/complete/fn.escaped_transform.html
pub fn parse_decoded_string(input: Z80Span) -> IResult<Z80Span, String, VerboseError<Z80Span>> {
    map(parse_string, |s| {
        s.replace("\\\\", "\\")
            .replace("\\a", &char::from(7).to_string())
            .replace("\\b", &char::from(8).to_string())
            .replace("\\t", "\t")
            .replace("\\r", "\r")
            .replace("\\n", "\n")
            .replace("\\v", &char::from(11).to_string())
            .replace("\\f", &char::from(12).to_string())
    })(input)
}

/// Get a factor
pub fn factor(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
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
                    parse_expr_bracketed_list,
                    // Manage functions
                    map(parse_word("RND()"), |_| Expr::Rnd),
                    parse_unary_function_call,
                    parse_binary_function_call,
                    parse_duration,
                    parse_assemble,
                    parse_any_function_call,
                    // manage values
                    alt((positive_number, negative_number)),
                    char_expr,
                    map(parse_decoded_string, |s| Expr::String(s.into())),
                    parse_counter,
                    // manage $
                    map(tag("$$"), |_x| Expr::Label(SmolStr::from("$$"))),
                    map(tag("$"), |_x| Expr::Label(SmolStr::from("$"))),
                    parse_bool_expr,
                    prefixed_label_expr,
                    // manage labels
                    map(parse_label(false), Expr::Label),
                    parens
                ))
            ),
            space0
        )
    )(input)?;

    let factor = match neg {
        Some(_) => Expr::Neg(factor.into()),
        None => factor
    };

    let factor = match not {
        Some(_) => Expr::BinaryNot(factor.into()),
        None => factor
    };

    Ok((input, factor))
}

pub fn negative_number(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(preceded(tag("-"), positive_number), |exp| {
        match exp {
            Expr::Value(v) => Expr::Value(-v),
            _ => unreachable!()
        }
    })(input)
}

pub fn positive_number(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(
        terminated(
            alt((hex_number_inner, bin_number_inner, dec_number_inner)),
            not(one_of(
                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#@_"
            ))
        ),
        |d: u32| Expr::Value(d as i32)
    )(input)
}

pub fn parse_labelprefix(input: Z80Span) -> IResult<Z80Span, LabelPrefix> {
    alt((
        value(LabelPrefix::Pageset, tag_no_case("{pageset}")),
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}"))
    ))(input)
}

fn fold_exprs(initial: Expr, remainder: Vec<(Oper, Expr)>) -> Expr {
    remainder.into_iter().fold(initial, |acc, pair| {
        let (oper, expr) = pair;
        match oper {
            Oper::Add => Expr::Add(Box::new(acc), Box::new(expr)),
            Oper::Sub => Expr::Sub(Box::new(acc), Box::new(expr)),
            Oper::Mul => Expr::Mul(Box::new(acc), Box::new(expr)),
            Oper::Div => Expr::Div(Box::new(acc), Box::new(expr)),
            Oper::Mod => Expr::Mod(Box::new(acc), Box::new(expr)),
            Oper::RightShift => Expr::RightShift(Box::new(acc), Box::new(expr)),
            Oper::LeftShift => Expr::LeftShift(Box::new(acc), Box::new(expr)),

            Oper::BinaryAnd => Expr::BinaryAnd(Box::new(acc), Box::new(expr)),
            Oper::BinaryOr => Expr::BinaryOr(Box::new(acc), Box::new(expr)),
            Oper::BinaryXor => Expr::BinaryXor(Box::new(acc), Box::new(expr)),

            Oper::BooleanAnd => Expr::BooleanAnd(Box::new(acc), Box::new(expr)),
            Oper::BooleanOr => Expr::BooleanOr(Box::new(acc), Box::new(expr)),

            Oper::Equal => Expr::Equal(Box::new(acc), Box::new(expr)),
            Oper::Different => Expr::Different(Box::new(acc), Box::new(expr)),
            Oper::StrictlyGreater => Expr::StrictlyGreater(Box::new(acc), Box::new(expr)),
            Oper::StrictlyLower => Expr::StrictlyLower(Box::new(acc), Box::new(expr)),
            Oper::LowerOrEqual => Expr::LowerOrEqual(Box::new(acc), Box::new(expr)),
            Oper::GreaterOrEqual => Expr::GreaterOrEqual(Box::new(acc), Box::new(expr))
        }
    })
}

/// Compute operations related to * % /
pub fn term(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = factor(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(factor, "*", Oper::Mul),
        parse_oper(factor, "%", Oper::Mod),
        parse_oper(factor, "/", Oper::Div)
    )))(input)?;

    Ok((input, fold_exprs(initial, remainder)))
}

/// Generate a parser of comparison symbol
/// inner: the function the parse the right operand of the symbol
/// pattern: the pattern to match in the source code
/// symbol: the symbol corresponding to the operation
fn parse_oper<F>(
    inner: F,
    pattern: &'static str,
    symbol: Oper
) -> impl Fn(Z80Span) -> IResult<Z80Span, (Oper, Expr), VerboseError<Z80Span>>
where
    F: Fn(Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>>
{
    move |input: Z80Span| {
        let (input, _) = space0(input)?;
        let (input, _) = tag_no_case(pattern)(input)?;
        let (input, _) = space0(input)?;
        let (input, operation) = inner(input)?;

        Ok((input, (symbol, operation)))
    }
}

fn parse_bool<F>(
    inner: F,
    pattern: &'static str,
    symbol: Oper
) -> impl Fn(Z80Span) -> IResult<Z80Span, (Oper, Expr), VerboseError<Z80Span>>
where
    F: Fn(Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>>
{
    move |input: Z80Span| {
        let (input, _) = space0(input)?;
        let (input, _) = tag_no_case(pattern)(input)?;
        let (input, _) = space0(input)?;
        let (input, operation) = inner(input)?;

        Ok((input, (symbol, operation)))
    }
}

/// Parse an expression
pub fn expr2(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = shift(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(shift, "<=", Oper::LowerOrEqual),
        parse_oper(shift, "<", Oper::StrictlyLower),
        parse_oper(shift, ">=", Oper::GreaterOrEqual),
        parse_oper(shift, ">", Oper::StrictlyGreater),
        parse_oper(shift, "==", Oper::Equal),
        parse_oper(shift, "!=", Oper::Different)
    )))(input)?;

    Ok((input, fold_exprs(initial, remainder)))
}

pub fn expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = expr2(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(expr2, "&&", Oper::BooleanAnd),
        parse_oper(expr2, "||", Oper::BooleanOr)
    )))(input)?;
    Ok((input, fold_exprs(initial, remainder)))
}

/// parse functions with one argument
pub fn parse_unary_function_call(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, func) = alt((
        value(
            UnaryFunction::High,
            alt((parse_word("HIGH"), parse_word("HI")))
        ),
        value(
            UnaryFunction::Low,
            alt((parse_word("LOW"), parse_word("LO")))
        ),
        value(
            UnaryFunction::Memory,
            alt((parse_word("PEEK"), parse_word("MEMORY")))
        ),
        value(UnaryFunction::Floor, parse_word("FLOOR")),
        value(UnaryFunction::Ceil, parse_word("CEIL")),
        value(UnaryFunction::Frac, parse_word("FRAC")),
        value(UnaryFunction::Int, parse_word("INT")),
        value(UnaryFunction::Sin, parse_word("SIN")),
        value(UnaryFunction::Cos, parse_word("COS")),
        value(UnaryFunction::ASin, parse_word("ASIN")),
        value(UnaryFunction::ACos, parse_word("ACOS")),
        value(UnaryFunction::Abs, parse_word("ABS")),
        value(UnaryFunction::Ln, parse_word("LN")),
        value(UnaryFunction::Log10, parse_word("LOG10")),
        value(UnaryFunction::Exp, parse_word("EXP")),
        value(UnaryFunction::Sqrt, parse_word("SQRT"))
    ))(input)?;

    let (input, exp) = cut(context(
        "UNARY function: error in parameters",
        delimited(
            tuple((space0, tag("("), space0)),
            expr,
            tuple((space0, tag(")")))
        )
    ))(input)?;

    Ok((input, Expr::UnaryFunction(func, Box::new(exp))))
}

/// parse functions with two arguments
pub fn parse_binary_function_call(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, func) = alt((
        value(BinaryFunction::Min, tag_no_case("MIN")),
        value(BinaryFunction::Max, tag_no_case("MAX")),
        value(BinaryFunction::Pow, tag_no_case("POW"))
    ))(input)?;

    let (input, _) = tuple((space0, tag("("), space0))(input)?;

    let (input, arg1) = expr(input)?;
    let (input, _) = tuple((space0, tag(","), space0))(input)?;
    let (input, arg2) = expr(input)?;

    let (input, _) = tuple((space0, tag(")")))(input)?;

    Ok((
        input,
        Expr::BinaryFunction(func, Box::new(arg1), Box::new(arg2))
    ))
}

pub fn parse_any_function_call(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, function_name) = parse_label(false)(input)?;
    let (input, arguments) = delimited(
        tuple((/* space0, */ tag("("), my_space0)),
        separated_list0(parse_comma, expr),
        tuple((my_space0, tag(")")))
    )(input)?;

    Ok((input, Expr::AnyFunction(function_name, arguments)))
}

/// Parser for functions taking into argument a token
pub fn token_function<'a>(
    function_name: &'static str
) -> impl Fn(Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    move |input: Z80Span| {
        let (input, _) = tuple((tag_no_case(function_name), space0, char('('), space0))(input)?;

        let (input, token) = parse_token(input)?;
        let token = token.token().clone(); // remove the location

        let (input, _) = tuple((space0, tag(")")))(input)?;

        Ok((input, token.unwrap().clone()))
    }
}

/// Parse the duration function
pub fn parse_duration(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, token) = token_function("duration")(input)?;
    Ok((input, Expr::Duration(Box::new(token))))
}

/// Parse the single opcode assembling function
pub fn parse_assemble(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, token) = token_function("opcode")(input)?;
    Ok((input, Expr::OpCode(Box::new(token))))
}

pub fn shift(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = comp(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(comp, "<<", Oper::LeftShift),
        parse_oper(comp, ">>", Oper::RightShift)
    )))(input)?;
    Ok((input, fold_exprs(initial, remainder)))
}

/// Parse operation related to + - & |
pub fn comp(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = term(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(term, "+", Oper::Add),
        parse_oper(term, "-", Oper::Sub),
        parse_oper(term, "&", Oper::BinaryAnd), /* TODO check if it works and not compete with && */
        parse_oper(term, "AND", Oper::BinaryAnd),
        parse_oper(term, "|", Oper::BinaryAnd), /* TODO check if it works and not compete with || */
        parse_oper(term, "OR", Oper::BinaryOr),
        parse_oper(term, "^", Oper::BinaryXor), /* TODO check if it works and not compete with ^^ */
        parse_oper(term, "XOR", Oper::BinaryXor)
    )))(input)?;
    Ok((input, fold_exprs(initial, remainder)))
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

#[cfg(test)]
mod test {
    use super::*;

    fn ctx() -> ParserContext {
        Default::default()
    }
    #[test]
    fn test_parse_end_directive() {
        let res = dbg!(parse_end_directive("endif".into()));
        assert!(res.is_ok());
    }
    #[test]
    fn parse_test_cond() {
        let res = dbg!(inner_code(Z80Span::new_extra(
            " nop
                endif"
                .to_owned(),
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
            parse_register_ixl(Z80Span::new_extra("ixl".to_owned(), ctx()))
                .unwrap()
                .1,
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert_eq!(
            parse_register_ixl(Z80Span::new_extra("lx".to_owned(), ctx()))
                .unwrap()
                .1,
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert!(parse_register_iyl(Z80Span::new_extra("ixl".to_owned(), ctx())).is_err());
    }

    #[test]
    fn test_parse_prefix_label() {
        let (span, res) =
            parse_labelprefix(Z80Span::new_extra("{bank}".to_owned(), ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(res, LabelPrefix::Bank);

        let (span, res) = dbg!(expr(Z80Span::new_extra("{bank}label".to_owned(), ctx())).unwrap());
        assert!(span.is_empty());
        assert_eq!(res, Expr::PrefixedLabel(LabelPrefix::Bank, "label".into()));
    }

    #[test]
    fn test_parse_expr_format() {
        let res = formatted_expr(Z80Span::new_extra("{hex} VAL".to_owned(), ctx()));
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
        let (span, res) = parse_print(Z80Span::new_extra("PRINT VAR".to_owned(), ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Raw(Expr::Label("VAR".into()))])
        );

        let (span, res) =
            parse_print(Z80Span::new_extra("PRINT VAR, VAR".to_owned(), ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![
                FormattedExpr::Raw(Expr::Label("VAR".into())),
                FormattedExpr::Raw(Expr::Label("VAR".into()))
            ])
        );

        let (span, res) =
            parse_print(Z80Span::new_extra("PRINT {hex}VAR".to_owned(), ctx())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Formatted(
                ExprFormat::Hex(None),
                Expr::Label("VAR".into())
            )])
        );

        let (span, res) =
            parse_print(Z80Span::new_extra("PRINT \"hello\"".to_owned(), ctx())).unwrap();
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
        let res = parse_repeat(Z80Span::new_extra(z80.to_owned(), ctx()));
        assert!(res.is_ok(), "{:?}", res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn parser_regression_1() {
        let res = std::dbg!(parse_ld_normal(Z80Span::new_extra(
            "ld a, chessboard_file".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", res);
    }
    #[test]
    fn parser_regression_1a() {
        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(parse_z80_line_complete(Z80Span::new_extra(
            &code.to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert!(res.unwrap().0.trim().is_empty());
    }
    #[test]
    fn parser_regression_1b() {
        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(parse_z80_line(Z80Span::new_extra(&code.to_owned(), ctx())));
        assert!(res.is_ok(), "{:?}", &res);
        assert!(res.unwrap().0.trim().is_empty());
    }
    #[test]
    fn parser_regression_1c() {
        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(many0(parse_z80_line)(Z80Span::new_extra(
            &code.to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.len(), 0, "{:?}", &res);
    }
    #[test]
    fn parser_regression_1d() {
        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = inner_code(Z80Span::new_extra(&code.to_owned(), ctx()));
        assert!(res.is_ok(), "{}", &res.err().unwrap().to_string());
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", &res);
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
                        "
            .to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
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
                        
                        endif"
                .to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn parser_regression2() {
        let res = std::dbg!(parse_assert(Z80Span::new_extra("assert (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)".to_owned(), ctx())));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn parser_sna() {
        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA V2".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA V3".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA V4".to_owned(),
            ctx()
        )));
        assert!(res.is_err(), "{:?}", &res);
    }

    #[test]
    fn test_parse_snaset() {
        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET Z80_SP, 0x500".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET GA_PAL, 0, 30".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET CRTC_REG, 1, 48".to_owned(),
            ctx()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn test_parse_r16_to_r8() {
        let res = parse_z80_line(Z80Span::new_extra(" ld a, hl.low".to_owned(), ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = parse_ld_normal(Z80Span::new_extra("ld bc.low, a".to_owned(), ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let (span, res) = res.unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::new_opcode(
                Mnemonic::Ld,
                Some(Register8::C.into()),
                Some(Register8::A.into()),
            )
        );

        let res = parse_z80_line(Z80Span::new_extra(" ld bc.low, a".to_owned(), ctx()));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let (span, res) = res.unwrap();
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

        let res = parse_z80_line(Z80Span::new_extra(
            "\t\tld  bc.low, a\n\t".to_owned(),
            ctx()
        ));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }
}
