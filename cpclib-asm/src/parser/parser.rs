#![allow(clippy::cast_lossless)]
use std::cell::RefCell;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::ops::Deref;
use std::rc::Rc;

use cpclib_sna::parse::bin_number;
use cpclib_sna::parse::dec_number;
use cpclib_sna::parse::hex_number;
use cpclib_sna::parse::parse_flag;
use cpclib_sna::parse::parse_flag_value;
use cpclib_sna::FlagValue;
use itertools::Itertools;
use nom_locate::LocatedSpan;

use nom::branch::*;
use nom::bytes::complete::tag;
use nom::bytes::complete::tag_no_case;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::separated_list1;
use nom::multi::*;
use nom::sequence::*;

#[allow(missing_docs)]
use nom::*;

use super::*;
use crate::preamble::*;
use cpclib_sna::SnapshotVersion;
use nom::lib::std::convert::Into;

use super::obtained::Locate;

/// ...
pub mod error_code {
    /// ...
    pub const ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION: u32 = 128;
    /// ...
    pub const INVALID_ARGUMENT: u32 = 129;
    /// ...
    pub const UNABLE_TO_PARSE_INNER_CONTENT: u32 = 130;
}

const FIRST_DIRECTIVE: &[&str] = &["IF", "IFDEF", "IFNDEF", "REPEAT", "REPT", "REP", "PHASE"];

// This table is supposed to contain the keywords that finish a section
const FINAL_DIRECTIVE: &[&str] = &[
    "REND",
    "ENDR",
    "ENDREPEAT",
    "ENDREP", // repeat directive
    "DEPHASE",
    "REND",  // rorg directive
    "ENDIF", // if directive
    "ELSE",
];
pub fn parse_z80_strrc_with_contextrc(
    code: Rc<String>,
    ctx: Rc<ParserContext>,
) -> Result<LocatedListing, AssemblerError> {
    let span = Z80Span::new_extra_from_rc(code, ctx);
    let mut listing = LocatedListing::new_empty_span(span);
    let ctx = listing.ctx();
    match parse_z80_code(listing.span()) {
        Err(e) => match e {
            nom::Err::Error(e) | Err::Failure(e) => {
                return Err(AssemblerError::SyntaxError { error: e });
            }
            nom::Err::Incomplete(_) => {
                return Err(AssemblerError::BugInParser {
                    error: "Bug in the parser".to_owned(),
                    context: ctx.deref().clone(),
                });
            }
        },

        Ok((remaining, mut parsed)) => {
            if remaining.len() > 0 {
                return Err(AssemblerError::BugInParser {
                    error: format!(
                        "Bug in the parser. The remaining source has not been assembled:\n{}",
                        remaining.deref()
                    ),
                    context: ctx.deref().clone(),
                });
            }

            if ctx.read_referenced_files {
                let errors = parsed.listing_mut()./*par_*/iter_mut()
                .map(|token|
                    token.read_referenced_file(&ctx)
                ).filter(
                    Result::is_err
                )
                .map(
                    Result::err
                )
                .map(
                    Option::unwrap
                )
                .collect::<Vec<_>>();
                if errors.len() > 0 {
                    return Err(AssemblerError::MultipleErrors { errors });
                }
            }

            return Ok(parsed);
        }
    }
}

/// Produce the stream of tokens. In case of error, return an explanatory string.
/// In case of success loop over all the tokens in order to expand those that read files
pub fn parse_z80_str_with_context<S: Into<String>>(
    str: S,
    ctx: ParserContext,
) -> Result<LocatedListing, AssemblerError> {
    parse_z80_strrc_with_contextrc(Rc::new(str.into()), Rc::new(ctx))
}

/// Parse a string and return the corresponding listing
pub fn parse_z80_str<S: Into<String>>(code: S) -> Result<LocatedListing, AssemblerError> {
    parse_z80_str_with_context(code, DEFAULT_CTX.clone())
}

/// nom many0 does not seem to fit our parser requirements
pub fn my_many0<O, E, F>(mut f: F) -> impl FnMut(Z80Span) -> IResult<Z80Span, Vec<O>, E>
where
    F: Parser<Z80Span, O, E>,
    E: ParseError<Z80Span>,
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
    input: Z80Span,
) -> IResult<Z80Span, LocatedListing, VerboseError<Z80Span>> {
    let (input, tokens) = many0(parse_z80_line)(input)?; // here it is my_many0 supposed to be used
    if input.is_empty() {
        let tokens = tokens.iter().flatten().cloned().collect_vec();
        Ok((
            input.clone(),
            tokens
                .try_into()
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
        ))
    } else {
        // Everything should have been consumed
        return Err(Err::Error(
            nom::error::ParseError::<Z80Span>::from_error_kind(input, ErrorKind::Many0),
        ));
    }
}

/// Parse a single line of Z80. Code useing directive on several lines cannot work
pub fn parse_z80_line(
    input: Z80Span,
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    let before_elem = input.clone();
    let (input2, tokens) = tuple((
        context("[DBG]not eof", not(eof)),
        alt((
            context("[DBG]empty line", parse_empty_line),
            context(
                "[DBG]macro only",
                delimited(
                    space1,
                    alt((
                        map(
                            alt((context("macro", parse_macro), context("basic", parse_basic))),
                            |t| vec![t.locate(before_elem.clone())],
                        ),
                        map(
                            alt((
                                context("repeat", parse_repeat),
                                context("rorg", parse_rorg),
                                context("[DBG] condition", parse_conditional),
                            )),
                            |lt| vec![lt],
                        ),
                    )),
                    preceded(space0, alt((line_ending, eof, tag(":")))),
                ),
            ),
            context("[DBG] line with label only", parse_z80_line_label_only),
            context("[DBG] standard line", parse_z80_line_complete),
        )),
    ))(input)?;

    Ok((input2, tokens.1))
}

/// Workaround because many0 is not used in the main root function
fn inner_code(input: Z80Span) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    context(
        "[DBG] inner code",
        fold_many0(parse_z80_line, Vec::new(), |mut inner, tokens| {
            inner.extend_from_slice(&tokens);
            inner
        }),
    )(input)
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
            rorg_start,
        ),
    ))
}

/// TODO
pub fn parse_macro(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = preceded(space0, parse_instr("MACRO"))(input)?;

    // macro name
    let (input, name) = context("MACRO: wrong name", cut(parse_label(false)))(input)?; // TODO use a specific function for that

    // macro arguments
    let (input, arguments) = opt(preceded(
        parse_comma, // comma after macro name is not mandatory
        separated_list1(
            parse_comma,
            /*parse_label(false)*/
            take_till(|c| c == '\n' || c == ':' || c == ',' || c == ' '),
        ),
    ))(input)?;

    let (input, content) = context(
        "MACRO: issue in the content",
        cut(preceded(
            space0,
            many_till(
                take(1usize),
                alt((tag_no_case("ENDM"), tag_no_case("MEND"))),
            ),
        )),
    )(input)?;

    Ok((
        input,
        Token::Macro(
            name,
            if arguments.is_some() {
                arguments
                    .unwrap()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            } else {
                Vec::new()
            },
            content
                .0
                .iter()
                .map(|s| -> String { s.to_string() })
                .collect::<String>(),
        ),
    ))
}

/// TODO
pub fn parse_repeat(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let repeat_start = input.clone();
    let (input, _) = preceded(
        space0,
        alt((
            parse_instr("REPEAT"),
            parse_instr("REPT"),
            parse_instr("REP"),
        )),
    )(input)?;

    let (input, count) = cut(context("REPEAT: wrong number of iterations", expr))(input)?;
    let (input, counter) = cut(context("REPEAT: issue in the counter", 
    opt(preceded(parse_comma, parse_label(false)))
    ))(input)?;
    let (input, counter_start) = opt(
        preceded(
            parse_comma,
            expr
        )
    )(input)?;
    let (input, inner) = cut(context("REPEAT: issue in the content", inner_code))(input)?;

    let (input, _) = cut(context(
        "REPEAT: not closed",
        tuple((
            space0,
            alt((
                parse_instr("ENDREPEAT"),
                parse_instr("ENDREPT"),
                parse_instr("ENDREP"),
                parse_instr("ENDR"),
                parse_instr("REND"),
            )),
            space0,
        )),
    ))(input)?;

    Ok((
        input.clone(),
        LocatedToken::Repeat(
            count,
            LocatedListing::try_from(inner)
                .unwrap_or_else(|_| LocatedListing::new_empty_span(input)),
            counter,
            counter_start,
            repeat_start,
        ),
    ))
}

/// TODO
pub fn parse_basic(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = tuple((space0, tag_no_case("LOCOMOTIVE"), space0))(input)?;

    let (input, args) = opt(separated_list1(
        preceded(space0, char(',')),
        preceded(space0, map(parse_label(false), |s| s.to_string())),
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
        preceded(space0, map(dec_number_inner, |d| d as u16)),
    )(input)
}
pub fn dec_number_inner(input: Z80Span) -> IResult<Z80Span, u32, VerboseError<Z80Span>> {
    let input_inner = input.deref().clone();
    let (input, number) = dec_number(input_inner).map_err(|err| {
        nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric,
        ))
    })?;

    Ok((Z80Span(input), number))
}
pub fn bin_number_inner(input: Z80Span) -> IResult<Z80Span, u32, VerboseError<Z80Span>> {
    let input_inner = input.deref().clone();
    let (input, number) = bin_number(input_inner).map_err(|err| {
        nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric,
        ))
    })?;

    Ok((Z80Span(input), number))
}
pub fn hex_number_inner(input: Z80Span) -> IResult<Z80Span, u32, VerboseError<Z80Span>> {
    let input_inner = input.deref().clone();
    let (input, number) = hex_number(input_inner).map_err(|err| {
        nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric,
        ))
    })?;

    Ok((Z80Span(input), number))
}

pub fn parse_flag_value_inner(
    input: Z80Span,
) -> IResult<Z80Span, FlagValue, VerboseError<Z80Span>> {
    let inner_input = input.deref().clone();

    let (input, number) = parse_flag_value(inner_input).map_err(|err| {
        nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric,
        ))
    })?;

    Ok((Z80Span(input), number))
}

/// TODO - currently consume several lines. Should do it only one time
pub fn parse_empty_line(
    input: Z80Span,
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    // let (input, _) = opt(line_ending)(input)?;
    let before_comment = input.clone();
    let (input, comment) = delimited(space0, opt(comment), space0)(input)?;
    let (input, _) = alt((line_ending, eof))(input)?;

    let mut res = Vec::new();
    if comment.is_some() {
        res.push(comment.unwrap().locate(before_comment));
    }

    Ok((input, res))
}

fn parse_single_token(
    first: bool,
) -> impl Fn(Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    move |input: Z80Span| {
        // Do not match ':' for the first case
        let input = if first {
            input
        } else {
            let (input, _) =
                context("[DBG] delimitation", delimited(space0, char(':'), space0))(input)?;
            input
        };

        // Get the token
        let (input, opcode) = context(
            "[DBG] single token",
            preceded(space0, alt((parse_token, parse_directive))),
        )(input)?;

        Ok((input, opcode))
    }
}

fn eof(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    if input.len() == 0 {
        Ok((input.clone(), input))
    } else {
        Err(Err::Error(error_position!(input, ErrorKind::Eof)))
    }
}

/// Parse a line
/// TODO add an argument o manage cases like '... : ENDIF'
pub fn parse_z80_line_complete(
    input: Z80Span,
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    // Eat previous line ending
    let (input, _) = opt(line_ending)(input)?;

    // Eat optional label
    let before_label = input.clone();
    let (input, label) = opt(parse_label(true))(input)?;
    let (input, _) = space1(input)?;

    // First directive MUST not be the  a keyword that ends a structure
    let (input, _) = not(parse_forbidden_keyword)(input)?;

    // Eat first token or directive
    let (input, opcode) = context("[DBG] first token", cut(parse_single_token(true)))(input)?;

    // Eat the additional opcodes
    let (input, additional_opcodes) = context(
        "[DBG] other tokens",
        cut(fold_many0(
            parse_single_token(false),
            Vec::new(),
            |mut acc: Vec<_>, item| {
                acc.push(item);
                acc
            },
        )),
    )(input)?;

    // Eat final comment
    let (input, _) = space0(input)?;
    let before_comment = input.clone();
    let (input, comment) = opt(comment)(input)?;
    let (input, _) = space0(input)?;

    // Ensure it is the end of line of file
    let (input, _) = cut(alt((line_ending, eof)))(input)?;

    // Build the result
    let mut tokens = Vec::new();
    if label.is_some() {
        tokens.push(Token::Label(label.unwrap()).locate(before_label));
    }
    tokens.push(opcode);
    for opcode in additional_opcodes {
        tokens.push(opcode);
    }
    if comment.is_some() {
        tokens.push(comment.unwrap().locate(before_comment));
    }

    Ok((input, tokens))
}

/// No opcodes are expected there.
/// Initially it was supposed to manage lines with only labels, however it has been extended
/// to labels fallowed by specific commands.
pub fn parse_z80_line_label_only(
    input: Z80Span,
) -> IResult<Z80Span, Vec<LocatedToken>, VerboseError<Z80Span>> {
    let before_label = input.clone();
    let (input, label) = parse_label(true)(input)?;

    // TODO make these stuff alternatives ...
    // Manage Equ
    // BUG Equ and = are supposed to be different
    let (input, equ) = opt(preceded(
        preceded(space1, alt((tag_no_case("DEFL"), tag_no_case("EQU"), tag_no_case("=")))),
        preceded(space1, expr),
    ))(input)?;

    // opt!(char!(':')) >>

    let before_comment = input.clone();
    let (input, comment) = delimited(space0, opt(comment), alt((line_ending, eof)))(input)?;

    {
        let mut tokens = Vec::new();

        if equ.is_some() {
            tokens.push(Token::Equ(label, equ.unwrap()).locate(before_label));
        } else {
            tokens.push(Token::Label(label).locate(before_label));
        }
        if comment.is_some() {
            tokens.push(comment.unwrap().locate(before_comment));
        }

        Ok((input, tokens))
    }
}

/// Parser for file names in appropriate directives
pub fn parse_fname(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    alt((
        preceded(tag("\""), terminated(take_until("\""), take(1usize))),
        preceded(tag("'"), terminated(take_until("'"), take(1usize))),
    ))(input)
}

/// Parser for the include directive
pub fn parse_include(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let include_start = input.clone();
    let (input, fname) = preceded(tuple((tag_no_case("INCLUDE"), space1)), parse_fname)(input)?;

    Ok((
        input,
        LocatedToken::Include(fname.to_string(), RefCell::new(None), include_start),
    ))
}

/// Parse for the various binary include directives
pub fn parse_incbin(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, transformation) = alt((
        map(tag_no_case("INCBIN"), |_| BinaryTransformation::None),
        map(tag_no_case("INCEXO"), |_| BinaryTransformation::Exomizer),
        map(tag_no_case("INCL49"), |_| BinaryTransformation::Lz49),
        map(tag_no_case("INCAPU"), |_| BinaryTransformation::Aplib),
    ))(input)?;

    let (input, fname) = preceded(space1, parse_fname)(input)?;

    let (input, offset) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, length) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, _extended_offset) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, off) = opt(preceded(
        tuple((space0, char(','), space0)),
        tag_no_case("OFF"),
    ))(input)?;

    Ok((
        input,
        Token::Incbin {
            fname: fname.to_string(),
            offset,
            length,
            extended_offset: None,
            off: off.is_some(),
            content: None.into(),
            transformation,
        },
    ))
}

pub fn parse_save(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, filename) = preceded(tuple((tag_no_case("SAVE"), space1)), parse_fname)(input)?;

    let (input, address) = preceded(parse_comma, expr)(input)?;
    let (input, size) = preceded(parse_comma, expr)(input)?;

    let (input, save_type) = opt(preceded(
        parse_comma,
        alt((
            value(SaveType::Amsdos, tag_no_case("AMSDOS")),
            value(SaveType::Dsk, tag_no_case("DSK")),
        )),
    ))(input)?;

    let (input, dsk_filename) = if save_type.is_some() {
        opt(preceded(parse_comma, parse_fname))(input)?
    } else {
        (input, None)
    };

    let (input, side) = if dsk_filename.is_some() {
        opt(preceded(parse_comma, expr))(input)?
    } else {
        (input, None)
    };

    let filename = filename.to_string();
    let dsk_filename = dsk_filename.map(|s| s.to_string());

    Ok((
        input,
        Token::Save {
            filename,
            address,
            size,
            save_type,
            dsk_filename,
            side,
        },
    ))
}

/// Parse  UNDEF directive.
pub fn parse_undef(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, label) =
        preceded(tuple((tag_no_case("UNDEF"), space1)), parse_label(false))(input)?;

    Ok((input, Token::Undef(label)))
}

/// Parse the opcodes. TODO rename as parse_opcode ...
pub fn parse_token(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
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
        parse_im,
    ))(input.clone())
    .map(|(i, r)| (i, r.locate(input)))
}

/// Parse ex af, af' instruction
pub fn parse_ex_af(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    value(
        Token::new_opcode(Mnemonic::ExAf, None, None),
        tuple((
            tag_no_case("EX"),
            space1,
            parse_register_af,
            parse_comma,
            tag_no_case("AF'"),
        )),
    )(input)
}

/// Parse ex hl, de instruction
pub fn parse_ex_hl_de(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    value(
        Token::new_opcode(Mnemonic::ExHlDe, None, None),
        alt((
            tuple((
                tag_no_case("EX"),
                space1,
                parse_register_hl,
                parse_comma,
                parse_register_de,
            )),
            tuple((
                tag_no_case("EX"),
                space1,
                parse_register_de,
                parse_comma,
                parse_register_hl,
            )),
        )),
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
        alt((parse_register_hl, parse_indexregister16)),
    ))(input)?;

    Ok((
        input,
        Token::new_opcode(Mnemonic::ExMemSp, Some(destination.8), None),
    ))
}

/// Parse any directive
pub fn parse_directive(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let dir_start = input.clone();
    alt((
        parse_include,
        map(
            alt((
                parse_assert,
                parse_bankset,
                parse_bank,
                parse_align,
                parse_breakpoint,
                parse_buildsna,
                parse_org,
                parse_defs,
                parse_incbin,
                parse_limit,
                parse_db_or_dw,
                parse_print,
                parse_protect,
                parse_run,
                parse_snaset,
                parse_save,
                parse_stable_ticker,
                parse_struct,
                parse_undef,
                parse_noarg_directive,
                parse_macro_call, // need to be the very last one as it eats everything else
            )),
            move |t| t.locate(dir_start.clone()),
        ),
    ))(input.clone())
}

/// Parse directives with no arguments
pub fn parse_noarg_directive(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    alt((
        value(Token::List, tag_no_case("list")),
        value(Token::NoList, tag_no_case("nolist")),
    ))(input)
}

#[derive(Clone, Copy, Debug)]
enum KindOfConditional {
    If,
    IfNot,
    IfDef,
    IfNdef,
}

/// Parse if expression.TODO finish the implementation in order to have ELSEIF and ELSE branches"
pub fn parse_conditional(input: Z80Span) -> IResult<Z80Span, LocatedToken, VerboseError<Z80Span>> {
    let if_start = input.clone();
    // Gest the kind of test to do
    let (input, test_kind) = alt((
        value(KindOfConditional::If, parse_instr("IF")),
        value(KindOfConditional::IfNot, parse_instr("IFNOT")),
        value(KindOfConditional::IfDef, parse_instr("IFDEF")),
        value(KindOfConditional::IfNdef, parse_instr("IFNDEF")),
    ))(input)?;

    // Get the corresponding test
    let (input, cond) = context(
        "Condition error",
        cut(delimited(
            space0,
            parse_conditional_condition(test_kind),
            space0,
        )),
    )(input)?;

    let (input, _) = alt((line_ending, tag(":")))(input)?;

    let (input, code) = context("main case", cut(inner_code))(input)?;

    let else_input = input.clone();
    let (input, r#else) = context(
        "else",
        opt(preceded(
            delimited(
                space0,
                parse_instr("ELSE"),
                cut(opt(alt((terminated(space0, line_ending), tag(":"))))),
            ),
            context("else code", inner_code),
        )),
    )(input)?;

    let (input, _) = context(
        "end cond",
        tuple((
            cut(alt((space1, delimited(space0, tag(":"), space0)))),
            cut(preceded(space0, parse_instr("ENDIF"))),
        )),
    )(input)?;

    Ok((
        input,
        LocatedToken::If(
            vec![(cond, code.try_into().unwrap())],
            r#else.map(|v| {
                LocatedListing::try_from(v)
                    .unwrap_or_else(|_| LocatedListing::new_empty_span(else_input))
            }),
            if_start,
        ),
    ))
}

/// Read the condition part in the parse_conditional macro
fn parse_conditional_condition(
    code: KindOfConditional,
) -> impl Fn(Z80Span) -> IResult<Z80Span, TestKind, VerboseError<Z80Span>> {
    move |input: Z80Span| -> IResult<Z80Span, TestKind, VerboseError<Z80Span>> {
        match &code {
            KindOfConditional::If => map(expr, |e| TestKind::True(e))(input),

            KindOfConditional::IfNot => map(expr, |e| TestKind::False(e))(input),

            KindOfConditional::IfDef => {
                map(parse_label(false), |l| TestKind::LabelExists(l))(input)
            }

            KindOfConditional::IfNdef => {
                map(parse_label(false), |l| TestKind::LabelDoesNotExist(l))(input)
            }

            _ => unreachable!(),
        }
    }
}

/// Parse a breakpint instruction
pub fn parse_breakpoint(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(preceded(parse_instr("BREAKPOINT"), opt(expr)), |exp| {
        Token::Breakpoint(exp)
    })(input)
}

pub fn parse_bankset(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("bankset")(input)?;
    let (input, count) = expr(input)?;

    Ok((input, Token::Bankset(count)))
}

pub fn parse_buildsna(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    terminated(
        map(
            preceded(
                parse_instr("BUILDSNA"),
                cut(opt(alt((tag_no_case("V2"), tag_no_case("V3"))))),
            ),
            |v| {
                Token::BuildSna(match v {
                    Some(txt) => {
                        if txt.to_lowercase() == "v2" {
                            Some(SnapshotVersion::V2)
                        } else {
                            Some(SnapshotVersion::V3)
                        }
                    }
                    None => None,
                })
            },
        ),
        not(alphanumeric1),
    )(input)
}

pub fn parse_run(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, exp) = preceded(parse_instr("RUN"), expr)(input)?;
    let (input, ga) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;

    Ok((input, Token::Run(exp, ga)))
}


pub fn parse_limit(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, exp) = preceded(parse_instr("LIMIT"), expr)(input)?;

    Ok((input, Token::Limit(exp)))
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
                space1,
            )),
            parse_label(false),
        ),
        |name| Token::StableTicker(StableTickerAction::Start(name)),
    )(input)
}

/// Parse end of ticker
pub fn parse_stable_ticker_stop(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    value(
        Token::StableTicker(StableTickerAction::Stop),
        tuple((
            opt(tag_no_case("stable")),
            tag_no_case("ticker"),
            space1,
            tag_no_case("stop"),
        )),
    )(input)
}

pub fn parse_bank(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("bank")(input)?;
    let (input, count) = expr(input)?;

    Ok((input, Token::Bankset(count)))
}

/// Parse fake and real LD instructions
pub fn parse_ld(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    context(
        "[DBG] ld",
        alt((
            context("[DBG] fake ld", parse_ld_fake),
            context("[DBG] normal ld", parse_ld_normal),
        )),
    )(input)
}

/// Parse artifical LD instruction (would be replaced by several real instructions)
pub fn parse_ld_fake(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = tuple((tag_no_case("LD"), space1))(input)?;

    let (input, dst) = terminated(
        parse_register16,
        not(alt((tag_no_case(".low"), tag_no_case(".high")))),
    )(input)?;

    let (input, _) = tuple((space0, tag(","), space0))(input)?;

    let (input, src) = parse_register16(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Ld, Some(dst), Some(src))))
}

/// Parse the valids LD versions
pub fn parse_ld_normal(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = context("[DBG] ...", tuple((space0, parse_instr("LD"), space0)))(input)?;

    let (input, dst) = cut(context(
        LD_WRONG_DESTINATION,
        alt((
            parse_reg_address,
            parse_indexregister_with_index,
            parse_register_sp,
            terminated(
                parse_register16,
                not(alt((tag_no_case(".low"), tag_no_case(".high")))),
            ),
            parse_register8,
            parse_indexregister16,
            parse_indexregister8,
            parse_register_i,
            parse_register_r,
            parse_hl_address,
            parse_address,
        )),
    ))(input)?;

    let (input, _) = context("LD: missing comma", cut(parse_comma))(input)?;

    // src possibilities depend on dst
    let (input, src) = context(LD_WRONG_SOURCE, cut(parse_ld_normal_src(&dst)))(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Ld, Some(dst), Some(src))))
}

/// Parse the source of LD depending on its destination
fn parse_ld_normal_src(
    dst: &DataAccess,
) -> impl Fn(Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> + '_ {
    move |input: Z80Span| {
        if dst.is_register_sp() {
            alt((
                parse_register_hl,
                parse_indexregister16,
                parse_address,
                parse_expr,
            ))(input)
        } else if dst.is_address_in_register16() {
            // by construction is IS HL
            alt((parse_register8, parse_expr))(input)
        } else if dst.is_register16() | dst.is_indexregister16() {
            alt((parse_address, parse_expr))(input)
        } else if dst.is_register8() {
            // todo find a way to merge them together
            if dst.is_register_a() {
                alt((
                    parse_indexregister_with_index,
                    parse_reg_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8,
                    parse_register_i,
                    parse_register_r,
                    parse_expr,
                ))(input)
            } else {
                alt((
                    parse_indexregister_with_index,
                    parse_hl_address,
                    parse_address,
                    parse_register8,
                    parse_indexregister8,
                    parse_expr,
                ))(input)
            }
        } else if dst.is_indexregister8() {
            alt((
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
                parse_expr,
            ))(input)
        } else if dst.is_memory() {
            alt((
                parse_register16,
                parse_register8,
                parse_register_sp,
                parse_indexregister16,
            ))(input)
        } else if dst.is_address_in_register16() {
            parse_register8(input)
        } else if dst.is_indexregister_with_index() {
            alt((parse_register8, parse_expr))(input)
        } else if dst.is_register_i() || dst.is_register_r() {
            parse_register_a(input)
        } else {
            Err(nom::Err::Error(VerboseError::from_error_kind(
                input,
                ErrorKind::Alt,
            )))
        }
    }
}

/// Parse RES, SET and BIT instructions
pub fn parse_res_set_bit(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, res_or_set) = alt((
        value(Mnemonic::Res, tag_no_case("RES")),
        value(Mnemonic::Bit, tag_no_case("BIT")),
        value(Mnemonic::Set, tag_no_case("SET")),
    ))(input)?;

    let (input, bit) = preceded(space1, parse_expr)(input)?;

    let (input, _) = delimited(space0, tag(","), space0)(input)?;

    let (input, operand) = alt((
        parse_register8,
        parse_hl_address,
        parse_indexregister_with_index,
    ))(input)?;

    // Bit and Res can copy the result in a reg
    let (input, mut hidden_arg) = if res_or_set == Mnemonic::Bit {
        (input, None)
    } else {
        opt(preceded(parse_comma, parse_register8))(input)?
    };

    Ok((
        input,
        Token::OpCode(
            res_or_set,
            Some(bit),
            Some(operand),
            hidden_arg.map(|d| d.get_register8().unwrap()),
        ),
    ))
}

/// Parse CP tokens
pub fn parse_cp(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(
            parse_instr("CP"),
            alt((
                parse_register8,
                parse_indexregister8,
                parse_hl_address,
                parse_indexregister_with_index,
                parse_expr,
            )),
        ),
        |operand| Token::new_opcode(Mnemonic::Cp, Some(operand), None),
    )(input)
}

/// Parse DB DW directives
pub fn parse_db_or_dw(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, is_db) = alt((
        map(alt((parse_instr("BYTE"), parse_instr("DB"), parse_instr("DEFB"), parse_instr("DEFM"))), |_| true),
        map(alt((parse_instr("WORD"), parse_instr("DW"), parse_instr("DEFW"))), |_| false),
    ))(input)?;

    let (input, expr) = expr_list(input)?;
    Ok((
        input,
        if is_db {
            Token::Defb(expr)
        } else {
            Token::Defw(expr)
        },
    ))
}

// Fail if we do not read a forbidden keyword
pub fn parse_forbidden_keyword(input: Z80Span) -> IResult<Z80Span, String, VerboseError<Z80Span>> {
    let (input, _) = space0(input)?;
    let (input, name) = parse_label(false)(input)?;

    if !FINAL_DIRECTIVE
        .iter()
        .find(|&&a| a.to_lowercase() == name.to_lowercase())
        .is_some()
    {
        return Err(Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::AlphaNumeric,
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
                tuple((char(']'), space0)),
            ),
            |l| {
                MacroParam::List(
                    l.into_iter()
                        .map(|p| Box::new(p.clone()))
                        .collect::<Vec<_>>(),
                )
            },
        ),
        map(many1(none_of(",\r\n][")), |s| {
            MacroParam::Single(s.iter().collect::<String>())
        }),
    ))(input)
}

/// Manage the call of a macro.
/// TODO use parse_forbidden_keyword
pub fn parse_macro_call(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    // BUG: added because of parsing issues. Need to find why and remove ot
    let (input, _) = space0(input)?;
    let (input, name) = parse_label(false)(input)?;

    // Check if the macro name is allowed
    if FIRST_DIRECTIVE
        .iter()
        .chain(FINAL_DIRECTIVE.iter())
        .find(|&&a| a.to_lowercase() == name.to_lowercase())
        .is_some()
    {
        Err(Err::Failure(
            nom::error::ParseError::<Z80Span>::from_error_kind(input, ErrorKind::AlphaNumeric),
        ))
    } else {
        let (input, args) = alt((
            value(
                Default::default(),
                delimited(space0, tag_no_case("(void)"), space0),
            ),
            opt(alt((
                /*expr_list,  */ // initially a list of expression was used; now it is just plain strings
                separated_list1(tuple((tag(","), space0)), parse_macro_arg),
                map(tag_no_case("(void)"), |_| Vec::new()),
            ))),
        ))(input)?;

        Ok((input, Token::MacroCall(name, args.unwrap_or_default())))
    }
}

fn parse_instr(
    name: &'static str,
) -> impl Fn(Z80Span) -> IResult<Z80Span, (), VerboseError<Z80Span>> + 'static {
    move |input: Z80Span| map(tuple((tag_no_case(name), not(alpha1), space0)), |_| ())(input)
}

/// ...
pub fn parse_djnz(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(preceded(parse_instr("DJNZ"), parse_expr), |expr| {
        Token::new_opcode(Mnemonic::Djnz, Some(expr), None)
    })(input)
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
    let (input, _) = context("assert", preceded(space0, parse_instr("ASSERT")))(input)?;

    let (input, expr) = context("expr", expr)(input)?;

    let (input, comment) = context(
        "assert comment",
        opt(preceded(
            delimited(space0, tag(","), space0),
            delimited(tag("\""), take_until("\""), tag("\"")),
        )),
    )(input)?;

    Ok((input, Token::Assert(expr, comment.map(|s| s.to_string()))))
}

/// ...
pub fn parse_align(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
   
   let (input, boundary) = preceded(
       parse_instr("ALIGN"), expr)(input)?;
   let (input, fill) = opt(preceded(parse_comma, expr))(input)?;

   Ok((
       input,
       Token::Align(boundary, fill)
   ))
}

/// ...
pub fn parse_print(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(
            parse_instr("PRINT"),
            cut(separated_list1(
                delimited(space0, tag(","), space0),
                alt((
                    formatted_expr,
                    map(expr, FormattedExpr::from),
                    map(string_between_quotes, {
                        |s: Z80Span| FormattedExpr::from(Expr::String(s.to_string()))
                    }),
                )),
            )),
        ),
        |exps| Token::Print(exps),
    )(input)
}

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
fn formatted_expr(input: Z80Span) -> IResult<Z80Span, FormattedExpr, VerboseError<Z80Span>> {
    let (input, _) = char('{')(input)?;
    let (input, format) = cut(alt((
        value(ExprFormat::Int, tag_no_case("INT")),
        value(ExprFormat::Hex(Some(2)), tag_no_case("HEX4")),
        value(ExprFormat::Hex(Some(4)), tag_no_case("HEX8")),
        value(ExprFormat::Hex(Some(8)), tag_no_case("HEX2")),
        value(ExprFormat::Hex(None), tag_no_case("HEX")),
        value(ExprFormat::Bin(Some(8)), tag_no_case("BIN8")),
        value(ExprFormat::Bin(Some(16)), tag_no_case("BIN16")),
        value(ExprFormat::Bin(Some(32)), tag_no_case("BIN32")),
        value(ExprFormat::Bin(None), tag_no_case("BIN")),
    )))(input)?;
    let (input, _) = char('}')(input)?;

    let (input, _) = space0(input)?;

    let (input, exp) = expr(input)?;

    Ok((input, FormattedExpr::Formatted(format, exp)))
}

fn parse_comma(input: Z80Span) -> IResult<Z80Span, (), VerboseError<Z80Span>> {
    map(tuple((space0, tag(","), space0)), |_| ())(input)
}

/// ...
pub fn parse_protect(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, start) = preceded(parse_instr("PROTECT"), expr)(input)?;

    let (input, end) = preceded(parse_comma, expr)(input)?;

    Ok((input, Token::Protect(start, end)))
}

/// ...
pub fn parse_logical_operator(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, operator) = alt((
        value(Mnemonic::And, parse_instr("AND")),
        value(Mnemonic::Or, parse_instr("Or")),
        value(Mnemonic::Xor, parse_instr("Xor")),
    ))(input)?;

    let (input, operand) = cut(context(
        "logical operand",
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr,
        )),
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
        parse_expr,
    ))(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Sub, Some(operand), None)))
}

/// Par se the SBC instruction
pub fn parse_sbc(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = tag_no_case("SBC")(input)?;
    let (input, _) = space1(input)?;

    let (input, opera) = opt(terminated(
        alt((parse_register_a, parse_register_hl)),
        parse_comma,
    ))(input)?;

    let opera = opera.unwrap_or(DataAccess::Register8(Register8::A));

    let (input, operb) = if opera.is_register_a() {
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr,
        ))(input)
    } else {
        alt((parse_register16, parse_register_sp))(input)
    }?;

    Ok((
        input,
        Token::new_opcode(Mnemonic::Sbc, Some(opera), Some(operb)),
    ))
}

/// Parse ADC and ADD instructions
pub fn parse_add_or_adc_complete(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, add_or_adc) = alt((
        value(Mnemonic::Adc, tag_no_case("ADC")),
        value(Mnemonic::Add, tag_no_case("ADD")),
    ))(input)?;

    let (input, _) = space1(input)?;

    let (input, first) = alt((
        value(DataAccess::Register8(Register8::A), parse_register_a),
        value(DataAccess::Register16(Register16::Hl), parse_register_hl),
        parse_indexregister16,
    ))(input)?;

    let (input, _) = tuple((space0, tag(","), space0))(input)?;

    let (input, second) = if first.is_register8() {
        alt((
            parse_register8,
            parse_indexregister8,
            parse_hl_address,
            parse_indexregister_with_index,
            parse_expr,
        ))(input)
    } else if first.is_register16() {
        alt((parse_register16, parse_register_sp))(input) // Case for HL XXX AF is accepted whereas it is not the case in real life
    } else if first.is_indexregister16() {
        alt((
            parse_register_bc,
            parse_register_de,
            parse_register_hl,
            parse_register_sp,
            verify(parse_register_ix, |_| first.is_register_ix()),
            verify(parse_register_iy, |_| first.is_register_iy()),
        ))(input)
    } else {
        return Err(nom::Err::Error(VerboseError::from_error_kind(
            input,
            ErrorKind::Alt,
        )));
    }?;

    Ok((
        input,
        Token::new_opcode(add_or_adc, Some(first), Some(second)),
    ))
}

/// TODO Find a way to not duplicate code with complete version
pub fn parse_add_or_adc_shorten(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, add_or_adc) = alt((
        value(Mnemonic::Adc, parse_instr("ADC")),
        value(Mnemonic::Add, parse_instr("ADD")),
    ))(input)?;

    let (input, second) = alt((
        parse_register8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr,
    ))(input)?;

    Ok((
        input,
        Token::new_opcode(
            add_or_adc,
            Some(DataAccess::Register8(Register8::A)),
            Some(second),
        ),
    ))
}

/// ...
pub fn parse_push_n_pop(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, push_or_pop) = alt((
        value(Mnemonic::Push, parse_instr("PUSH")),
        value(Mnemonic::Pop, parse_instr("POP")),
    ))(input)?;

    let (input, registers) =
        separated_list1(parse_comma, alt((parse_register16, parse_indexregister16)))(input)?;

    if registers.len() > 1 {
        match push_or_pop {
            Mnemonic::Push => Ok((input, Token::MultiPush(registers))),
            Mnemonic::Pop => Ok((input, Token::MultiPop(registers))),
            _ => unreachable!(),
        }
    } else {
        Ok((
            input,
            Token::new_opcode(push_or_pop, Some(registers[0].clone()), None),
        ))
    }
}

/// ...
pub fn parse_ret(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(tag_no_case("RET"), opt(preceded(space1, parse_flag_test))),
        |cond| {
            Token::new_opcode(
                Mnemonic::Ret,
                if cond.is_some() {
                    Some(DataAccess::FlagTest(cond.unwrap()))
                } else {
                    None
                },
                None,
            )
        },
    )(input)
}

/// ...
pub fn parse_inc_dec(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, inc_or_dec) = alt((
        value(Mnemonic::Inc, parse_instr("INC")),
        value(Mnemonic::Dec, parse_instr("DEC")),
    ))(input)?;

    let (input, register) = alt((
        parse_register16,
        parse_indexregister16,
        parse_register8,
        parse_indexregister8,
        parse_register_sp,
        parse_hl_address,
        parse_indexregister_with_index,
    ))(input)?;

    Ok((input, Token::new_opcode(inc_or_dec, Some(register), None)))
}

/// TODO manage other out formats
pub fn parse_out(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("OUT")(input)?;

    // get the port proposal
    let (input, port) = alt((parse_portc, parse_portnn))(input)?;

    let (input, _) = parse_comma(input)?;

    // the vlaue depends on the port
    let (input, value) = if port.is_portc() {
        // reg c
        alt((
            parse_register8,
            value(DataAccess::from(Expr::from(0)), tag("0")),
        ))(input)?
    } else {
        parse_register_a(input)?
    };

    Ok((
        input,
        Token::new_opcode(Mnemonic::Out, Some(port), Some(value)),
    ))
}

/// Parse all the in flavors
pub fn parse_in(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("IN")(input)?;

    // get the port proposal
    let (input, destination) = cut(alt((
        parse_register8,
        value(DataAccess::from(Expr::from(0)), tag("0")),
    )))(input)?;
    // TODO portc only if first arg is not 0
    let (input, _) = cut(parse_comma)(input)?;
    let (input, port) = cut(alt((
        parse_portc,
        verify(parse_portnn, |_| {
            destination.get_register8().unwrap().is_a()
        }),
    )))(input)?;

    Ok((
        input,
        Token::new_opcode(Mnemonic::In, Some(destination), Some(port)),
    ))
}

/// Parse the rst instruction
pub fn parse_rst(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("RST")(input)?;
    let (input, val) = parse_expr(input)?;

    Ok((input, Token::new_opcode(Mnemonic::Rst, Some(val), None)))
}

/// Parse the IM instruction
pub fn parse_im(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("IM")(input)?;
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
    input: Z80Span,
) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, oper) = alt((
        value(Mnemonic::Rlc, parse_instr("RLC")),
        value(Mnemonic::Rrc, parse_instr("RRC")),
        value(Mnemonic::Rl, parse_instr("RL")),
        value(Mnemonic::Rr, parse_instr("RR")),
        value(Mnemonic::Sla, parse_instr("SLA")),
        value(Mnemonic::Sra, parse_instr("SRA")),
        value(Mnemonic::Srl, parse_instr("SRL")),
        value(Mnemonic::Sl1, parse_instr("SL1")),
    ))(input)?;

    let (input, arg) = alt((
        parse_register8,
        parse_hl_address,
        parse_indexregister_with_index,
    ))(input)?;

    // hidden opcodes
    let (input, arg2) = opt(preceded(parse_comma, parse_register8))(input)?;

    Ok((input, Token::new_opcode(oper, Some(arg), arg2)))
}

/// TODO reduce the flag space for jr"],
pub fn parse_call_jp_or_jr(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, call_jp_or_jr) = alt((
        value(Mnemonic::Jp, parse_instr("JP")),
        value(Mnemonic::Jr, parse_instr("JR")),
        value(Mnemonic::Call, parse_instr("CALL")),
    ))(input)?;

    let (input, flag_test) = opt(terminated(
        parse_flag_test,
        delimited(space0, tag(","), space0),
    ))(input)?;

    let (input, dst) = cut(context(
        match call_jp_or_jr {
            Mnemonic::Jp => JP_WRONG_PARAM,
            Mnemonic::Jr => JR_WRONG_PARAM,
            Mnemonic::Call => CALL_WRONG_PARAM,
            _ => unreachable!(),
        },
        alt((
            verify(
                alt((
                    parse_hl_address,
                    parse_indexregister_address,
                    parse_register_hl,
                    parse_indexregister16,
                )),
                |_| call_jp_or_jr.is_jp() && flag_test.is_none(),
            ), // not possible for call and for jp/jr when there is flag
            parse_expr,
        )),
    ))(input)?;

    // Allow to parse JP HL as to be JP (HL) original notation is misleading
    let dst = match dst {
        DataAccess::IndexRegister16(reg) => DataAccess::MemoryIndexRegister16(reg),
        DataAccess::Register16(reg) => DataAccess::MemoryRegister16(reg),
        other => other,
    };

    let flag_test = if flag_test.is_some() {
        Some(DataAccess::FlagTest(flag_test.unwrap()))
    } else {
        None
    };

    Ok((
        input,
        Token::new_opcode(call_jp_or_jr, flag_test, Some(dst)),
    ))
}

/// ...
pub fn parse_flag_test(input: Z80Span) -> IResult<Z80Span, FlagTest, VerboseError<Z80Span>> {
    alt((
        value(FlagTest::NZ, tag_no_case("NZ")),
        value(FlagTest::Z, tag_no_case("Z")),
        value(FlagTest::NC, tag_no_case("NC")),
        value(FlagTest::C, tag_no_case("C")),
        value(FlagTest::PO, tag_no_case("PO")),
        value(FlagTest::PE, tag_no_case("PE")),
        value(FlagTest::P, tag_no_case("P")),
        value(FlagTest::M, tag_no_case("M")),
    ))(input)
}

/*
/// XXX to remove as soon as possible
named_attr!(#[doc="TODO"],
parse_dollar <&str, Expr>, do_parse!(
    tag!("$") >>
    (Expr::Label(String::from("$")))
)
);
*/

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
pub fn parse_register16(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    alt((
        parse_register_hl,
        parse_register_bc,
        parse_register_de,
        parse_register_af,
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
                        value('H', tag_no_case("high")),
                    )),
                ),
                space0,
            )),
            |(r16, code, _)| {
                if code == 'L' {
                    r16.to_data_access_for_low_register().unwrap()
                } else {
                    r16.to_data_access_for_high_register().unwrap()
                }
            },
        ),
        parse_register_a,
        parse_register_b,
        parse_register_c,
        parse_register_d,
        parse_register_e,
        parse_register_h,
        parse_register_l,
    ))(input)
}

/// Parse register i
pub fn parse_register_i(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::SpecialRegisterI,
        tuple((tag_no_case("I"), not(alphanumeric1))),
    )(input)
}

/// Parse register r
pub fn parse_register_r(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::SpecialRegisterR,
        tuple((tag_no_case("R"), not(alphanumeric1))),
    )(input)
}

macro_rules! parse_any_register8 {
    ($name: ident, $char:expr, $reg:expr) => {
        /// Parse register $char
        pub fn $name(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
            value(DataAccess::Register8($reg), parse_instr($char))(input)
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
    register: Register16,
) -> impl for<'src, 'ctx> Fn(Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    move |input: Z80Span| {
        value(
            DataAccess::Register16(register),
            tuple((tag_no_case(representation), not(alphanumeric1))),
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
        tuple((tag_no_case("IX"), not(alphanumeric1))),
    )(input)
}

/// Parse the IY register
pub fn parse_register_iy(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::IndexRegister16(IndexRegister16::Iy),
        tuple((tag_no_case("IY"), not(alphanumeric1))),
    )(input)
}

// TODO find a way to not use that
macro_rules! parse_any_indexregister8 {
    ($($reg:ident, $alias:ident)*) => {$(
        paste::paste! {
            /// Parse register $reg
            pub fn [<parse_register_ $reg:lower>] (input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
                value(
                    DataAccess::IndexRegister8(IndexRegister8::$reg),
                    tuple((
                        alt((
                            tag_no_case( stringify!($reg)),
                            tag_no_case( stringify!($alias))
                        ))
                        , not(alphanumeric1)))
                    )(input)
                }
            }
        )*}
    }
parse_any_indexregister8!(Ixh,hx Ixl,lx Iyh,hy Iyl,ly);

/// Parse and indexed register in 8bits
pub fn parse_indexregister8(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    alt((
        parse_register_ixh,
        parse_register_iyh,
        parse_register_ixl,
        parse_register_iyl,
    ))(input)
}

/// Parse a 16 bits indexed register
pub fn parse_indexregister16(
    input: Z80Span,
) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    terminated(
        map(
            alt((
                map(tag_no_case("IX"), |_| IndexRegister16::Ix),
                map(tag_no_case("IY"), |_| IndexRegister16::Iy),
            )),
            |reg| DataAccess::IndexRegister16(reg),
        ),
        not(alphanumeric1),
    )(input)
}

/// Parse the use of an indexed register as (IX + 5)"
pub fn parse_indexregister_with_index(
    input: Z80Span,
) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    let (input, reg) = preceded(tuple((tag("("), space0)), parse_indexregister16)(input)?;

    let (input, op) = preceded(
        space0,
        alt((value(Oper::Add, tag("+")), value(Oper::Sub, tag("-")))),
    )(input)?;

    let (input, expr) = terminated(expr, tuple((space0, tag(")"))))(input)?;

    Ok((
        input,
        DataAccess::IndexRegister16WithIndex(
            reg.get_indexregister16().unwrap(),
            match op {
                Oper::Add => expr,
                Oper::Sub => expr.neg(),
                _ => unreachable!(),
            },
        ),
    ))
}

/// Parse (C) used in in/out
pub fn parse_portc(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::PortC,
        tuple((tag("("), space0, parse_register_c, space0, tag(")"))),
    )(input)
}

/// Parse (nn) used in in/out
pub fn parse_portnn(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    map(
        delimited(tag("("), expr, preceded(space0, tag(")"))),
        |address| DataAccess::PortN(address),
    )(input)
}

/// Parse an address access `(expression)`
pub fn parse_address(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    map(
        delimited(tag("("), expr, preceded(space0, tag(")"))),
        |address| DataAccess::Memory(address),
    )(input)
}

/// Parse (R16)
pub fn parse_reg_address(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    map(
        delimited(
            terminated(tag("("), space0),
            parse_register16,
            preceded(space0, tag(")")),
        ),
        |reg| DataAccess::MemoryRegister16(reg.get_register16().unwrap()),
    )(input)
}

/// Parse (HL)
pub fn parse_hl_address(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    value(
        DataAccess::MemoryRegister16(Register16::Hl),
        delimited(
            terminated(tag("("), space0),
            parse_register_hl,
            preceded(space0, tag(")")),
        ),
    )(input)
}

/// Parse (ix) and (iy)
pub fn parse_indexregister_address(
    input: Z80Span,
) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    map(
        delimited(
            terminated(tag("("), space0),
            parse_indexregister16,
            preceded(space0, tag(")")),
        ),
        |reg| DataAccess::MemoryIndexRegister16(reg.get_indexregister16().unwrap()),
    )(input)
}

/// Parse an expression and returns it inside a DataAccession::Expression
pub fn parse_expr(input: Z80Span) -> IResult<Z80Span, DataAccess, VerboseError<Z80Span>> {
    let (input, expr) = expr(input)?;
    Ok((input, DataAccess::Expression(expr)))
}

/// Parse standard org directive
pub fn parse_org(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = tuple((tag_no_case("ORG"), space1))(input)?;

    let (input, val1) = expr(input)?;
    let (input, val2) = opt(preceded(parse_comma, expr))(input)?;

    Ok((input, Token::Org(val1, val2)))
}

/// Parse defs instruction. TODO add optional parameters
pub fn parse_defs(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = alt(( parse_instr("FILL"), parse_instr("DS"), parse_instr("DEFS")))(input)?;

    let (input, val) = cut(context("Wrong argument", expr))(input)?;

    Ok((input, Token::Defs(val, None)))
}

/// Parse any opcode having no argument
pub fn parse_opcode_no_arg(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    alt((
        parse_opcode_no_arg1,
        parse_opcode_no_arg2,
        parse_opcode_no_arg3,
    ))(input)
}

fn parse_opcode_no_arg1(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, mnemonic) = alt((
        map(parse_instr("DI"), |_| Mnemonic::Di),
        map(parse_instr("CCF"), |_| Mnemonic::Ccf),
        map(parse_instr("EI"), |_| Mnemonic::Ei),
        map(parse_instr("EXX"), |_| Mnemonic::Exx),
        map(parse_instr("HALT"), |_| Mnemonic::Halt),
        map(parse_instr("LDIR"), |_| Mnemonic::Ldir),
        map(parse_instr("LDDR"), |_| Mnemonic::Lddr),
        map(parse_instr("LDI"), |_| Mnemonic::Ldi),
        map(parse_instr("LDD"), |_| Mnemonic::Ldd),
        map(parse_instr("NOPS2"), |_| Mnemonic::Nops2),
        map(parse_instr("NOP"), |_| Mnemonic::Nop),
        map(parse_instr("OUTD"), |_| Mnemonic::Outd),
        map(parse_instr("OUTI"), |_| Mnemonic::Outi),
    ))(input)?;

    Ok((input, Token::new_opcode(mnemonic, None, None)))
}

fn parse_opcode_no_arg2(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, mnemonic) = alt((
        value(Mnemonic::Rla, parse_instr("RLA")),
        value(Mnemonic::Rra, parse_instr("RRA")),
        value(Mnemonic::Rlca, parse_instr("RLCA")),
        value(Mnemonic::Scf, parse_instr("SCF")),
        value(Mnemonic::Ind, parse_instr("IND")),
        value(Mnemonic::Indr, parse_instr("INDR")),
        value(Mnemonic::Ini, parse_instr("INI")),
        value(Mnemonic::Inir, parse_instr("INIR")),
        value(Mnemonic::Reti, parse_instr("RETI")),
        value(Mnemonic::Retn, parse_instr("RETN")),
        value(Mnemonic::Rrca, parse_instr("RRCA")),
        value(Mnemonic::Cpd, parse_instr("CPD")),
        value(Mnemonic::Cpdr, parse_instr("CPDR")),
        value(Mnemonic::Cpi, parse_instr("CPI")),
        value(Mnemonic::Cpir, parse_instr("CPIR")),
        value(Mnemonic::Cpl, parse_instr("CPL")),
    ))(input)?;

    Ok((input, Token::new_opcode(mnemonic, None, None)))
}

fn parse_opcode_no_arg3(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, mnemonic) = alt((
        value(Mnemonic::Daa, parse_instr("DAA")),
        value(Mnemonic::Neg, parse_instr("NEG")),
        value(
            Mnemonic::Otdr,
            alt((parse_instr("OUTDR"), parse_instr("OTDR"))),
        ),
        value(
            Mnemonic::Otir,
            alt((parse_instr("OUTIR"), parse_instr("OTIR"))),
        ),
        value(Mnemonic::Rld, parse_instr("RLD")),
        value(Mnemonic::Rrd, parse_instr("RRD")),
    ))(input)?;

    Ok((input, Token::new_opcode(mnemonic, None, None)))
}

fn parse_struct(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("STRUCT")(input)?;
    let (input, name) = cut(parse_label(false))(input)?;

    let (input, _) = preceded(space0, line_ending)(input)?;

    let (input, fields) = many1(pair(
        verify(terminated(parse_label(false), space1), |label: &str| {
            label.to_ascii_lowercase() != "endstruct"
        }),
        cut(terminated(
            alt((parse_db_or_dw, parse_macro_call)),
            pair(space0, line_ending),
        )),
    ))(input)?;

    let (input, _) = cut(preceded(space0, parse_instr("ENDSTRUCT")))(input)?;

    Ok((input, Token::Struct(name.to_owned(), fields)))
}

fn parse_snaset(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    let (input, _) = parse_instr("SNASET")(input)?;

    let (input, flagname) = cut(context(SNASET_WRONG_LABEL, parse_label(false)))(input)?;
    let (input, _) = context(SNASET_MISSING_COMMA, cut(parse_comma))(input)?;

    let (input, values) = cut(separated_list1(
        delimited(space0, parse_comma, space0),
        parse_flag_value_inner,
    ))(input)?;

    let (flagname, value) = if values.len() == 1 {
        (flagname, values[0].clone())
    } else {
        (
            format!("{}:{}", flagname, values[0].as_u16().unwrap()),
            values[1].clone(),
        )
    };

    let (_, flag) = (parse_flag(LocatedSpan::new(&flagname))).map_err(|_e| {
        nom::Err::Error(VerboseError::from_error_kind(
            input.clone(),
            ErrorKind::AlphaNumeric,
        ))
    })?;
    Ok((input, Token::SnaSet(flag, value)))
}

/// Parse a comment that start by `;` and ends at the end of the line.
pub fn comment(input: Z80Span) -> IResult<Z80Span, Token, VerboseError<Z80Span>> {
    map(
        preceded(tag(";"), take_till(|ch| ch == '\n')),
        |string: Z80Span| Token::Comment(string.to_string()),
    )(input)
}

/// Usefull later for db
pub fn string_between_quotes(input: Z80Span) -> IResult<Z80Span, Z80Span, VerboseError<Z80Span>> {
    delimited(char('\"'), is_not("\""), char('\"'))(input)
}

/// TODO
pub fn string_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(string_between_quotes, |string| {
        Expr::String(string.to_string())
    })(input)
}

pub fn char_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(
        delimited(
            tag("'"), 
            nom::character::complete::anychar, 
            tag("'")
        ), 
        |c| {
        Expr::Char(c)
    })(input)
}

/// Parse a label(label: S)
pub fn parse_label(
    doubledots: bool,
) -> impl Fn(Z80Span) -> IResult<Z80Span, String, VerboseError<Z80Span>> {
    move |input: Z80Span| {
        // Get the label

        let (input, first) =
            one_of("@abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ._")(input)?;
        let (input, middle) = opt(
            is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.")
        )(input)?;

        if middle.is_none() && first == '@' || first == '.' {
            return Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)));
        }

        let input = if doubledots {
            let (input, _) = opt(tag_no_case(":"))(input)?;
            input
        } else {
            input
        };

        let label = format!(
            "{}{}", 
            first, 
            middle.map(|v| v.iter_elements().collect::<String>())
                .unwrap_or_default()
        );

        let impossible = ["af", "hl", "de", "bc", "ix", "iy", "ixl", "ixh"];
        if impossible.iter().any(|val| val == &label.to_lowercase()) {
            Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
        } else {
            Ok((input, label))
        }
    }
}

pub fn prefixed_label_expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, prefix) = alt((
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}")),
        value(LabelPrefix::Pageset, tag_no_case("{pageset}")),
    ))(input)?;
    let (input, label) = preceded(
        space0,
        alt((
            parse_label(false),
            map(tag_no_case("$"), |s: Z80Span| s.to_string()),
        )),
    )(input)?;

    Ok((input, Expr::PrefixedLabel(prefix, label)))
}

/*
/// Parse an ASM file an returns the stream of tokens.
pub fn parse_file(fname: String) -> Vec<Token> {
    let mut f = File::open(fnmae).expect(format!("{} not found", fname));
    let mut contents = String::new();
    f.read_to_string(&mut contents)
    .expect(format!("Something went wrong reading {}", fname));


    parse_binary_stream(fname.to_bytes())
}
*/

// XXX Code greatly inspired from https://github.com/Geal/nom/blob/master/tests/arithmetic_ast.rs

/// Read a value
pub fn parse_value(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let extra = input.extra.clone();
    let (input, val) = alt((hex_number, dec_number, bin_number))(input.clone().into())
        .map_err(|op| nom::Err::Error(VerboseError::from_error_kind(input, ErrorKind::ParseTo)))?;

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
            tag("}".into())
        ),
        |l| {
            Expr::Label(format!("{{{}}}", l))
        }
    )(input)
}

/// Read a parenthesed expression
pub fn parens(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    delimited(
        delimited(space0, tag("("), space0),
        map(map(expr, Box::new), Expr::Paren),
        delimited(space0, tag(")"), space0),
    )(input)
}

/// Get a factor
pub fn factor(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, neg) = opt(delimited(
        space0,
        tag("!"),
        space0
    ))(input)?;
    
    let (input, factor) = 
    context(
        "[DBG]factor",
        delimited(
            space0,
            context(
                "[DBG]alt",
                alt((
                    // Manage functions
                    parse_unary_functions,
                    parse_binary_functions,
                    parse_duration,
                    parse_assemble,
                    // manage values
                    alt((positive_number, negative_number)),
                    char_expr,
                    parse_counter,
                    // manage $
                    map(tag("$"), |_x| Expr::Label(String::from("$"))),
                    prefixed_label_expr,
                    // manage labels
                    map(parse_label(false), Expr::Label),
                    parens,
                )),
            ),
            space0,
        ),
    )(input)?;

    let factor = match neg {
        Some(_) => Expr::Neg(factor.into()),
        None => factor
    };

    Ok((input, factor))
}

pub fn negative_number(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(preceded(tag("-"), positive_number), |exp| match exp {
        Expr::Value(v) => Expr::Value(-v),
        _ => unreachable!(),
    })(input)
}

pub fn positive_number(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    map(
        alt((hex_number_inner, bin_number_inner, dec_number_inner)),
        |d: u32| Expr::Value(d as i32),
    )(input)
}

pub fn parse_labelprefix(input: Z80Span) -> IResult<Z80Span, LabelPrefix> {
    alt((
        value(LabelPrefix::Pageset, tag_no_case("{pageset}")),
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}")),
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
            Oper::GreaterOrEqual => Expr::GreaterOrEqual(Box::new(acc), Box::new(expr)),
        }
    })
}

/// Compute operations related to * % /
pub fn term(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = factor(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(factor, "*", Oper::Mul),
        parse_oper(factor, "%", Oper::Mod),
        parse_oper(factor, "/", Oper::Div),
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
    symbol: Oper,
) -> impl Fn(Z80Span) -> IResult<Z80Span, (Oper, Expr), VerboseError<Z80Span>>
where
    F: Fn(Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>>,
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
    symbol: Oper,
) -> impl Fn(Z80Span) -> IResult<Z80Span, (Oper, Expr), VerboseError<Z80Span>>
where
    F: Fn(Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>>,
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
pub fn expr(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = comp(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(comp, "<=", Oper::LowerOrEqual),
        parse_oper(comp, "<", Oper::StrictlyLower),
        parse_oper(comp, ">=", Oper::GreaterOrEqual),
        parse_oper(comp, ">", Oper::StrictlyGreater),
        parse_oper(comp, "==", Oper::Equal),
        parse_oper(comp, "!=", Oper::Different),
        parse_oper(comp, "&&", Oper::BooleanAnd),
        parse_oper(comp, "||", Oper::BooleanOr),
    )))(input)?;

    Ok((input, fold_exprs(initial, remainder)))
}

/// parse functions with one argument
pub fn parse_unary_functions(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, func) = alt((
        value(UnaryFunction::High, tag_no_case("HI")),
        value(UnaryFunction::Low, tag_no_case("LO")),
    ))(input)?;

    let (input, _) = tuple((space0, tag("("), space0))(input)?;

    let (input, exp) = expr(input)?;

    let (input, _) = tuple((space0, tag(")")))(input)?;

    Ok((input, Expr::UnaryFunction(func, Box::new(exp))))
}

/// parse functions with two arguments
pub fn parse_binary_functions(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, func) = alt((
        value(BinaryFunction::Min, tag_no_case("MIN")),
        value(BinaryFunction::Max, tag_no_case("MAX")),
    ))(input)?;

    let (input, _) = tuple((space0, tag("("), space0))(input)?;

    let (input, arg1) = expr(input)?;
    let (input, _) = tuple((space0, tag(","), space0))(input)?;
    let (input, arg2) = expr(input)?;

    let (input, _) = tuple((space0, tag(")")))(input)?;

    Ok((
        input,
        Expr::BinaryFunction(func, Box::new(arg1), Box::new(arg2)),
    ))
}

/// Parser for functions taking into argument a token
pub fn token_function<'a>(
    function_name: &'static str,
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

/// Parse operation related to + - & |
pub fn comp(input: Z80Span) -> IResult<Z80Span, Expr, VerboseError<Z80Span>> {
    let (input, initial) = term(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(term, "+", Oper::Add),
        parse_oper(term, "-", Oper::Sub),
        parse_oper(term, "&", Oper::BinaryAnd), // TODO check if it works and not compete with &&
        parse_oper(term, "AND", Oper::BinaryAnd),
        parse_oper(term, "|", Oper::BinaryAnd), // TODO check if it works and not compete with ||
        parse_oper(term, "OR", Oper::BinaryOr),
        parse_oper(term, "^", Oper::BinaryXor), // TODO check if it works and not compete with ^^
        parse_oper(term, "XOR", Oper::BinaryXor),
    )))(input)?;
    Ok((input, fold_exprs(initial, remainder)))
}

/// Generate a string from a parsing error. Probably deprecated
#[allow(clippy::needless_pass_by_value)]
pub fn decode_parsing_error(_orig: &str, _e: ::nom::Err<&str>) -> String {
    unimplemented!("pub fn decode_parsing_error(orig: &str, e: ::nom::Err<&str>) -> String")
    /*
    let error_string;

    if let ::nom::Err::Failure(::nom::simple_errors::Context::Code(
        remaining,
        ErrorKind::Custom(_),
    )) = e
    {
        let bytes = orig.as_bytes();
        let complete_size = orig.len();
        let remaining_size = remaining.input_len();
        let error_position = complete_size - remaining_size;
        let line_end = {
            let mut idx = error_position;
            while idx < complete_size && bytes[idx] != b'\n' {
                idx += 1;
            }
            idx
        };
        let line_start = {
            let mut idx = error_position;
            while idx > 0 && bytes[idx - 1] != b'\n' {
                idx -= 1;
            }
            idx
        };

        let line = &orig[line_start..line_end];
        let line_idx = orig[..(error_position)]
        .bytes()
        .filter(|b| *b == b'\n')
        .count(); // way too slow I guess
        let column_idx = error_position - line_start;
        let error_description = "Error because";
        let empty = iter::repeat(" ").take(column_idx).collect::<String>();
        error_string = format!(
            "{}:{}:{} {}\n{}\n{}^",
            "fname", line_idx, column_idx, error_description, line, empty
        );
    } else {
        error_string = String::from("Unknown error");
    }

    error_string
    */
}

#[cfg(test)]
mod test {
    use super::*;

    static CTX: ParserContext = ParserContext {
        context_name: None,
        current_filename: None,
        read_referenced_files: false,
        search_path: Vec::new(),
    };

    #[test]
    fn parse_test_cond() {
        let res = inner_code(Z80Span::new_extra(
            " nop
                endif"
                .to_owned(),
            CTX.clone(),
        ));
        assert!(res.is_ok());
        assert_eq!(res.unwrap().1.len(), 1);

        let res = inner_code(Z80Span::new_extra(
            " nop
                else",
            CTX.clone(),
        ));
        assert!(res.is_ok());
        assert_eq!(res.unwrap().1.len(), 1);

        let res = parse_conditional_condition(KindOfConditional::If)(Z80Span::new_extra(
            "THING",
            CTX.clone(),
        ));
        assert!(res.is_ok());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "if THING
                    nop
                    endif 
                    ",
            CTX.clone()
        ),));
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0.trim());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "if THING
                    nop
                    endif ",
            CTX.clone()
        ),));
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0.trim());

        let res = parse_conditional(Z80Span::new_extra(
            "if THING
                    nop
                    else
                    nop
                    endif",
            CTX.clone(),
        ));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = parse_conditional(Z80Span::new_extra(
            "ifndef THING
                    nop
                    else
                    nop
                    endif",
            CTX.clone(),
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
            CTX.clone()
        )));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = std::dbg!(parse_conditional(Z80Span::new_extra(
            "ifndef __DEFINED_DEBUG__
                    __DEFINED_DEBUG__ equ 1
                    endif",
            CTX.clone()
        )));
        assert!(res.is_ok());
        assert_eq!(b"", res.unwrap().0.as_bytes());

        let res = std::dbg!(parse_z80_line(Z80Span::new_extra(
            " ifndef __DEFINED_DEBUG__
                    __DEFINED_DEBUG__ equ 1
                    endif",
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", res);
        assert_eq!(b"", res.unwrap().0.as_bytes());
    }

    #[test]
    fn parse_indexregister8() {
        assert_eq!(
            parse_register_ixl(Z80Span::new_extra("ixl".to_owned(), CTX.clone()))
                .unwrap()
                .1,
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert_eq!(
            parse_register_ixl(Z80Span::new_extra("lx".to_owned(), CTX.clone()))
                .unwrap()
                .1,
            DataAccess::IndexRegister8(IndexRegister8::Ixl)
        );

        assert!(parse_register_iyl(Z80Span::new_extra("ixl".to_owned(), CTX.clone())).is_err());
    }

    #[test]
    fn test_parse_prefix_label() {
        let (span, res) =
            parse_labelprefix(Z80Span::new_extra("{bank}".to_owned(), CTX.clone())).unwrap();
        assert!(span.is_empty());
        assert_eq!(res, LabelPrefix::Bank);

        let (span, res) = expr(Z80Span::new_extra("{bank}label".to_owned(), CTX.clone())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Expr::PrefixedLabel(LabelPrefix::Bank, "label".to_string())
        );
    }

    #[test]
    fn test_parse_expr_format() {
        let res = formatted_expr(Z80Span::new_extra("{hex} VAL".to_owned(), CTX.clone()));
        assert!(res.is_ok());
        let (span, res) = res.unwrap();
        assert!(span.is_empty());

        assert_eq!(
            res,
            FormattedExpr::Formatted(ExprFormat::Hex(None), Expr::Label("VAL".to_string()))
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
        let (span, res) =
            parse_print(Z80Span::new_extra("PRINT VAR".to_owned(), CTX.clone())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Raw(Expr::Label("VAR".to_string()))])
        );

        let (span, res) =
            parse_print(Z80Span::new_extra("PRINT VAR, VAR".to_owned(), CTX.clone())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![
                FormattedExpr::Raw(Expr::Label("VAR".to_string())),
                FormattedExpr::Raw(Expr::Label("VAR".to_string()))
            ])
        );

        let (span, res) =
            parse_print(Z80Span::new_extra("PRINT {hex}VAR".to_owned(), CTX.clone())).unwrap();
        assert!(span.is_empty());
        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Formatted(
                ExprFormat::Hex(None),
                Expr::Label("VAR".to_string())
            )])
        );

        let (span, res) = parse_print(Z80Span::new_extra(
            "PRINT \"hello\"".to_owned(),
            CTX.clone(),
        ))
        .unwrap();
        assert!(span.is_empty());

        assert_eq!(
            res,
            Token::Print(vec![FormattedExpr::Raw(Expr::String("hello".to_string()))])
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
        let res = parse_repeat(Z80Span::new_extra(z80.to_owned(), CTX.clone()));
        assert!(res.is_ok(), "{:?}", res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn parser_regression_1() {
        let res = std::dbg!(parse_ld_normal(Z80Span::new_extra(
            "ld a, chessboard_file".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", res);

        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(parse_z80_line_complete(Z80Span::new_extra(
            &code.to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert!(res.unwrap().0.trim().is_empty());

        let code = " nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(parse_z80_line(Z80Span::new_extra(
            &code.to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert!(res.unwrap().0.trim().is_empty());

        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(many0(parse_z80_line)(Z80Span::new_extra(
            &code.to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.len(), 0, "{:?}", &res);

        let code = " nop
                    nop
                    "
        .replace("\u{C2}\u{A0}", " ");
        let res = std::dbg!(inner_code(Z80Span::new_extra(
            &code.to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.len(), 0, "{:?}", &res);

        let res = std::dbg!(parse_z80_str(
            "
                        ld a, chessboard_file
                        jp .common_part_loading_in_main_memory
                        "
        ));
 
        assert!(res.is_ok(), "{:?}", &res);
     //   assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

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
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

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
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn parser_regression2() {
        let res = std::dbg!(parse_assert(Z80Span::new_extra("assert (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)".to_owned(), CTX.clone())));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn parser_sna() {
        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA V2".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA V3".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_buildsna(Z80Span::new_extra(
            "BUILDSNA V4".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_err(), "{:?}", &res);
    }

    #[test]
    fn test_parse_snaset() {
        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET Z80_SP, 0x500".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET GA_PAL, 0, 30".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = std::dbg!(parse_snaset(Z80Span::new_extra(
            "SNASET CRTC_REG, 1, 48".to_owned(),
            CTX.clone()
        )));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }

    #[test]
    fn test_parse_r16_to_r8() {
        let res = parse_z80_line(Z80Span::new_extra(" ld a, hl.low".to_owned(), CTX.clone()));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);

        let res = parse_ld_normal(Z80Span::new_extra("ld bc.low, a".to_owned(), CTX.clone()));
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

        let res = parse_z80_line(Z80Span::new_extra(" ld bc.low, a".to_owned(), CTX.clone()));
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
            CTX.clone(),
        ));
        assert!(res.is_ok(), "{:?}", &res);
        assert_eq!(res.clone().unwrap().0.trim().len(), 0, "{:?}", res);
    }
}
