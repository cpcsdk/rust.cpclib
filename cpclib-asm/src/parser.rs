#![allow(clippy::cast_lossless)]

use nom::branch::*;
use nom::bytes::complete::tag;
use nom::bytes::complete::tag_no_case;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::separated_nonempty_list;
use nom::multi::*;
use nom::sequence::*;

use either::*;
#[allow(missing_docs)]
use nom::*;
use std::path::PathBuf;

use std::str::FromStr;

use crate::preamble::*;
use cpclib_sna::parse::*;

use rayon::prelude::*;
/// ...
pub mod error_code {
    /// ...
    pub const ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION: u32 = 128;
    /// ...
    pub const INVALID_ARGUMENT: u32 = 129;
    /// ...
    pub const UNABLE_TO_PARSE_INNER_CONTENT: u32 = 130;
}

/// Context information that can guide the parser
/// TODO add assembling flags
#[derive(Default, Clone, Debug)]
pub struct ParserContext {
    /// Filename that is currently parsed
    current_filename: Option<PathBuf>,
    /// Search path to find files
    search_path: Vec<PathBuf>,
}

#[allow(missing_docs)]
impl ParserContext {
    pub fn set_current_filename<P: Into<PathBuf>>(&mut self, file: P) {
        let file = file.into();
        self.current_filename = Some(file.canonicalize().unwrap_or(file))
    }

    /// Add a search path and ensure it is ABSOLUTE
    pub fn add_search_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.search_path.push(path.into().canonicalize().unwrap())
    }

    /// Add the folder that contains the given file. Ignore if there are issues with the filename
    pub fn add_search_path_from_file<P: Into<PathBuf>>(&mut self, file: P) {
        let path = file.into().canonicalize();
        if let Ok(path) = path {
            let path = path.parent().unwrap().to_owned();
            self.add_search_path(path);
        }
    }

    /// Return the real path name that correspond to the requested file
    pub fn get_path_for<P: Into<PathBuf>>(&self, fname: P) -> Option<PathBuf> {
        let fname = fname.into();

        // We expect the file to exists if no search_path is provided
        if self.search_path.is_empty() {
            if fname.is_file() {
                return Some(fname);
            } else {
                return None;
            }
        } else {
            // loop over all possibilities
            for search in &self.search_path {
                let current_path = search.join(fname.clone());

                if current_path.is_file() {
                    return Some(current_path);
                }
            }
        }

        // No file found
        None
    }
}

const FIRST_DIRECTIVE: &[&str] = &[
    "IF", "IFDEF", "IFNDEF",
    "REPEAT", "REPT", "REP",
    "PHASE"
];

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

/// Produce the stream of tokens. In case of error, return an explanatory string.
/// In case of success loop over all the tokens in order to expand those that read files
pub fn parse_str_with_context(code: &str, ctx: &ParserContext) -> Result<Listing, AssemblerError> {
    match parse_z80_code(code.into()) {
        Err(e) => Err(AssemblerError::SyntaxError {
            error: format!("Error while parsing: {:?}", e), //TODO add context
        }),
        Ok((remaining, mut parsed)) => {
            if remaining.len() > 0 {
                Err(AssemblerError::BugInParser {
                    error: format!(
                        "Bug in the parser. The remaining source has not been assembled:\n{}",
                        remaining
                    ),
                    context: ctx.clone(),
                })
            } else {
                let errors = parsed.listing_mut()./*par_*/iter_mut()
                .map(|token|
                    token.read_referenced_file(ctx)
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

                Ok(parsed)
            }
        }
    }
}

/// Parse a string and return the corresponding listing
pub fn parse_str(code: &str) -> Result<Listing, AssemblerError> {
    parse_str_with_context(code, &ParserContext::default())
}

/// nom many0 does not seem to fit our parser requirements
pub fn my_many0<'a, O, E, F>(f: F) -> impl Fn(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
    E: ParseError<&'a str>,
{
    move |i: &'a str| {
        let mut acc = Vec::with_capacity(4);
        let mut i = i.clone();
        loop {
            match f(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, acc)),
                Err(e) => return Err(e),
                Ok((i1, o)) => {
                    if i1 == i {
                        return Ok((i, acc));
                    }

                    i = i1;
                    acc.push(o);
                }
            }
        }
    }
}

/// Parse a complete code
pub fn parse_z80_code(input: &str) -> IResult<&str, Listing, VerboseError<&str>> {
    let (input, tokens) = my_many0(parse_z80_line)(input)?;
    if input.is_empty() {
        let mut res: Vec<Token> = Vec::new();
        for list in tokens {
            res.extend(list);
        }

        Ok((input, res.into()))
    } else {
        // Everything should have been consumed
        return Err(Err::Error(nom::error::ParseError::<&str>::from_error_kind(
            input,
            ErrorKind::Many0,
        )));
    }
}

/// For an unknwon reason, the parse_z80_code function fails when there is no comment...
/// // Mainly used for test
/// This one is a workaround as still as the other is not fixed
// RODO ASAP #[deprecated]
pub fn parse_z80_str(code: &str) -> IResult<&str, Listing, VerboseError<&str>> {
    let mut tokens = Vec::new();
    let mut rest = None;
    let src = "<str>";

    for (line_number, line) in code.split('\n').enumerate() {
        let res = parse_z80_line(line);
        match res {
            Ok((res, local_tokens)) => {
                tokens.extend_from_slice(&local_tokens);
                rest = Some(res);
            }
            Err(e) => {
                let error_string = format!("Error at line {}: {}", line_number, line);
                eprintln!("{}:{} ({}) {}", src, line_number, line, error_string);
                return Err(e);
            }
        }
    }
    Ok((rest.unwrap(), tokens.into()))
}

/// Parse a single line of Z80. Code useing directive on several lines cannot work
pub fn parse_z80_line(input: &str) -> IResult<&str, Vec<Token>, VerboseError<&str>> {
    let (input2, tokens) = 
    //tuple((
       // not(parse_forbidden_keyword),
        alt((
            context("empty line", parse_empty_line),
            context("repeat", map(parse_repeat, { |repeat| vec![repeat] })),
            context("macro", map(parse_macro, { |m| vec![m] })),
            context("basic", map(parse_basic, { |basic| vec![basic] })),
            context("rorg", map(parse_rorg, { |rorg| vec![rorg] })),
            context("condition", map(preceded(space1, parse_conditional), { |cond| vec![cond] })),
            context("line with label only", parse_z80_line_label_only),
            context("standard line", parse_z80_line_complete),
        ))
    //))
    (input)?;

    Ok((input2, std::dbg!(tokens)))
}

/// Workaround because many0 is not used in the main root function
fn inner_code(input: &str) -> IResult<&str, Vec<Token>, VerboseError<&str>> {
    map(
        many0(
            parse_z80_line
        ), 
        |tokens| {
        let mut inner: Vec<Token> = Vec::new();
        for group in &tokens {
            inner.extend_from_slice(&group);
        }
        inner
    })(input)
}

/// TODO
pub fn parse_rorg(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = space0(input)?;
    let (input, _) = alt((tag_no_case("PHASE"), tag_no_case("RORG")))(input)?;

    let (input, exp) = delimited(space1, expr, space0)(input)?;

    let (input, _) = line_ending(input)?;

    let (input, inner) = inner_code(input)?;

    let (input, _) = preceded(space0, alt((tag_no_case("DEPHASE"), tag_no_case("REND"))))(input)?;

    Ok((input, Token::Rorg(exp, inner.into())))
}

/// TODO
pub fn parse_macro(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = delimited(space0, tag_no_case("MACRO"), space1)(input)?;

    // macro name
    let (input, name) = parse_label(false)(input)?; // TODO use a specific function for that

    // macro arguments
    let (input, arguments) = opt(preceded(
        tuple((space0, char(','), space0)),
        separated_nonempty_list(
            tuple((space0, char(','), space0)),
            /*parse_label(false)*/
            take_till(|c| c == '\n' || c == ':'),
        ),
    ))(input)?;

    let (input, content) = preceded(space0, many_till(take(1usize), tag_no_case("ENDM")))(input)?;

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
pub fn parse_repeat(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = preceded(
        space0,
        alt((
            parse_instr("REPEAT"),
            parse_instr("REPT"),
            parse_instr("REP"),
        ))
    )(input)?;

    eprintln!("rep");

    let (input, count) = cut(context("repeat count", expr))(input)?;

    let (input, inner) = cut(context("repeat content", inner_code))(input)?;

    let (input, _) = cut(context("repeat closure", tuple((
        space0,
        alt((
            parse_instr("ENDREPEAT"),
            parse_instr("ENDREPT"),
            parse_instr("ENDREP"),
            parse_instr("ENDR"),
            parse_instr("REND"),
        )),
        space0,
    ))))
    (input)?;

    Ok((input, Token::Repeat(count, BaseListing::from(inner), None)))
}

/// TODO
pub fn parse_basic(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tuple((space0, tag_no_case("LOCOMOTIVE"), space0))(input)?;

    let (input, args) = opt(separated_nonempty_list(
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
pub fn parse_basic_hide_lines(input: &str) -> IResult<&str, Vec<u16>, VerboseError<&str>> {
    let (input, _) = tuple((tag_no_case("HIDE_LINES"), space1))(input)?;
    separated_nonempty_list(
        preceded(space0, char(',')), 
        preceded(space0, dec_number)
    )(input)
}

/// TODO - currently consume several lines. Should do it only one time
pub fn parse_empty_line(input: &str) -> IResult<&str, Vec<Token>, VerboseError<&str>> {
    // let (input, _) = opt(line_ending)(input)?;
    let (input, comment) = delimited(space0, opt(comment), space0)(input)?;
    let (input, _) = alt((line_ending, eof))(input)?;

    let mut res = Vec::new();
    if comment.is_some() {
        res.push(comment.unwrap());
    }

    Ok((input, res))
}

fn parse_single_token(first: bool) -> impl Fn(&str) -> IResult<&str, Token, VerboseError<&str>> {
    move |input: &str| {
        // Do not match ':' for the first case
        let input = if first {
            input
        } else {
            let (input, _) = delimited(space0, char(':'), space0)(input)?;
            input
        };

        // Get the token
        let (input, opcode) = alt((parse_token, parse_directive))(input)?;

        Ok((input, opcode))
    }
}

fn eof(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    if input.len() == 0 {
        Ok((input, input))
    } else {
        Err(Err::Error(error_position!(input, ErrorKind::Eof)))
    }
}

/// Parse a line
/// TODO add an argument o manage cases like '... : ENDIF'
pub fn parse_z80_line_complete(input: &str) -> IResult<&str, Vec<Token>, VerboseError<&str>> {
    // Eat previous line ending
    let (input, _) = opt(line_ending)(input)?;

    // Eat optional label
    let (input, label) = opt(parse_label(true))(input)?;
    let (input, _) = space1(input)?;

    // First directive MUST not be the ned of a keyword
    let (input, _) = not(parse_forbidden_keyword)(input)?;

    // Eat first token or directive
    let (input, opcode) = parse_single_token(true)(input)?;

    // Eat the additional opcodes
    let (input, additional_opcodes) = fold_many0(
        parse_single_token(false),
        Vec::new(),
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        },
    )(input)?;

    // Eat final comment
    let (input, _) = space0(input)?;
    let (input, comment) = opt(comment)(input)?;

    // Ensure it is the end of line of file
    let (input, _) = alt((line_ending, eof))(input)?;

    // Build the result
    let mut tokens = Vec::new();
    if label.is_some() {
        tokens.push(Token::Label(label.unwrap()));
    }
    tokens.push(opcode);
    for opcode in additional_opcodes {
        tokens.push(opcode);
    }
    if comment.is_some() {
        tokens.push(comment.unwrap());
    }

    Ok((input, tokens))
}

/// No opcodes are expected there.
/// Initially it was supposed to manage lines with only labels, however it has been extended
/// to labels fallowed by specific commands.
pub fn parse_z80_line_label_only(input: &str) -> IResult<&str, Vec<Token>, VerboseError<&str>> {
    let (input, label) = preceded(opt(line_ending), parse_label(true))(input)?;

    // TODO make these stuff alternatives ...
    // Manage Equ
    // BUG Equ and = are supposed to be different
    let (input, equ) = opt(preceded(
        preceded(space1, alt((tag_no_case("EQU"), tag_no_case("=")))),
        preceded(space1, expr),
    ))(input)?;

    // opt!(char!(':')) >>

    let (input, comment) = delimited(space0, opt(comment), alt((line_ending, eof)))(input)?;

    {
        let mut tokens = Vec::new();

        if equ.is_some() {
            tokens.push(Token::Equ(label, equ.unwrap()));
        } else {
            tokens.push(Token::Label(label));
        }
        if comment.is_some() {
            tokens.push(comment.unwrap());
        }

        Ok((input, tokens))
    }
}

/// Parser for file names in appropriate directives
pub fn parse_fname(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    alt((
        preceded(tag("\""), terminated(take_until("\""), take(1usize))),
        preceded(tag("'"), terminated(take_until("'"), take(1usize))),
    ))(input)
}

/// Parser for the include directive
pub fn parse_include(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, fname) = preceded(tuple((tag_no_case("INCLUDE"), space1)), parse_fname)(input)?;

    Ok((input, Token::Include(fname.to_string(), None)))
}

/// Parse for the various binary include directives
pub fn parse_incbin(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, transformation) = alt((
        map(tag_no_case("INCBIN"), |_| BinaryTransformation::None),
        map(tag_no_case("INCEXO"), |_| BinaryTransformation::Exomizer),
        map(tag_no_case("INCL49"), |_| BinaryTransformation::Lz49),
    ))(input)?;

    let (input, fname) = preceded(space1, parse_fname)(input)?;

    let (input, offset) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, length) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
    let (input, extended_offset) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;
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
            content: None,
            transformation,
        },
    ))
}

/// Parse  UNDEF directive.
pub fn parse_undef(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, label) =
        preceded(tuple((tag_no_case("UNDEF"), space1)), parse_label(false))(input)?;

    Ok((input, Token::Undef(label)))
}

/// Parse the opcodes. TODO rename as parse_opcode ...
pub fn parse_token(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
    ))(input)
}

/// Parse ex af, af' instruction
pub fn parse_ex_af(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    value(
        Token::OpCode(Mnemonic::ExAf, None, None),
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
pub fn parse_ex_hl_de(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    value(
        Token::OpCode(Mnemonic::ExHlDe, None, None),
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
pub fn parse_ex_mem_sp(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
        Token::OpCode(Mnemonic::ExMemSp, Some(destination.8), None),
    ))
}

/// Parse any directive
pub fn parse_directive(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    alt((
        parse_assert,
        parse_align,
        parse_breakpoint,
        parse_org,
        parse_defs,
        parse_include,
        parse_incbin,
        parse_db_or_dw,
        parse_print,
        parse_protect,
        parse_run,
        parse_snaset,
        parse_stable_ticker,
        parse_undef,
        parse_noarg_directive,
        parse_macro_call, // need to be the very last one as it eats everything else
    ))(input)
}

/// Parse directives with no arguments
pub fn parse_noarg_directive(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
pub fn parse_conditional(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    // Gest the kind of test to do
    let (input, test_kind) = alt((
        value(KindOfConditional::If, parse_instr("IF")),
        value(KindOfConditional::IfNot, parse_instr("IFNOT")),
        value(KindOfConditional::IfDef, parse_instr("IFDEF")),
        value(KindOfConditional::IfNdef, parse_instr("IFNDEF")),
    ))(input)?;

    // Get the corresponding test
    let (input, cond) = cut(context(
        "Condition error",
        delimited(space0, parse_conditional_condition(test_kind), space0),
    ))(input)?;


    let (input, _) = alt((line_ending, tag(":")))(input)?;

    let (input, code) = cut(context("main case", inner_code))(input)?;

    let (input, r#else) = opt(preceded(
        delimited(
            space0,
            parse_instr("ELSE"),
            cut(opt(alt((terminated(space0, line_ending), tag(":"))))),
        ),
        context("else code", inner_code),
    ))(input)?;

    if r#else.is_some() {
        eprintln!("ELSE met");
    }


    let (input, _) = tuple((
        cut(alt((space1, delimited(space0, tag(":"), space0)))),
        cut(delimited(
            space0,
            map(parse_instr("ENDIF"), |_| println!("ENDIF met")),
            cut(alt((terminated(space0, opt(line_ending)), tag(":")))),
        )),
    ))(input)?;

    Ok((
        input,
        Token::If(vec![(cond, code.into())], r#else.map(BaseListing::from)),
    ))
}

/// Read the condition part in the parse_conditional macro
fn parse_conditional_condition(
    code: KindOfConditional,
) -> impl Fn(&str) -> IResult<&str, TestKind, VerboseError<&str>> {
    move |input: &str| -> IResult<&str, TestKind, VerboseError<&str>> {
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
pub fn parse_breakpoint(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    map(preceded(tag_no_case("BREAKPOINT"), opt(expr)), |exp| {
        Token::Breakpoint(exp)
    })(input)
}

pub fn parse_run(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, exp) = preceded(tag_no_case("RUN"), expr)(input)?;
    let (input, ga) = opt(preceded(tuple((space0, char(','), space0)), expr))(input)?;

    Ok((input, Token::Run(exp, ga)))
}

/// Parse tickin directives
pub fn parse_stable_ticker(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    alt((parse_stable_ticker_start, parse_stable_ticker_stop))(input)
}

/// Parse begining of ticker
pub fn parse_stable_ticker_start(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
pub fn parse_stable_ticker_stop(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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

/// Parse fake and real LD instructions
pub fn parse_ld(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    alt((parse_ld_fake, parse_ld_normal))(input)
}

/// Parse artifical LD instruction (would be replaced by several real instructions)
pub fn parse_ld_fake(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tuple((tag_no_case("LD"), space1))(input)?;

    let (input, dst) = parse_register16(input)?;

    let (input, _) = tuple((space0, tag(","), space0))(input)?;

    let (input, src) = parse_register16(input)?;

    Ok((input, Token::OpCode(Mnemonic::Ld, Some(dst), Some(src))))
}

/// Parse the valids LD versions
pub fn parse_ld_normal(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tuple((space0, tag_no_case("LD"), space1))(input)?;

    let (input, dst) = alt((
        parse_reg_address,
        parse_indexregister_with_index,
        parse_register_sp,
        parse_register16,
        parse_register8,
        parse_indexregister16,
        parse_indexregister8,
        parse_register_i,
        parse_register_r,
        parse_hl_address,
        parse_address,
    ))(input)?;

    let (input, _) = tuple((space0, tag(","), space0))(input)?;

    // src possibilities depend on dst
    let (input, src) = parse_ld_normal_src(&dst)(input)?;

    Ok((input, Token::OpCode(Mnemonic::Ld, Some(dst), Some(src))))
}

/// Parse the source of LD depending on its destination
fn parse_ld_normal_src(
    dst: &DataAccess,
) -> impl Fn(&str) -> IResult<&str, DataAccess, VerboseError<&str>> + '_ {
    move |input: &str| {
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
            Err(nom::Err::Error(VerboseError::from_error_kind(input, ErrorKind::Alt)))
        }
    }
}

/// Parse RES, SET and BIT instructions
pub fn parse_res_set_bit(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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

    Ok((input, Token::OpCode(res_or_set, Some(bit), Some(operand))))
}

/// Parse CP tokens
pub fn parse_cp(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
        |operand| Token::OpCode(Mnemonic::Cp, Some(operand), None),
    )(input)
}

/// Parse DB DW directives
pub fn parse_db_or_dw(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, is_db) = alt((
        map(alt((parse_instr("DB"), parse_instr("DEFB"))), { |_| true }),
        map(alt((parse_instr("DW"), parse_instr("DEFW"))), { |_| false }),
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
pub fn parse_forbidden_keyword(input: &str) -> IResult<&str, String, VerboseError<&str>>  {
    let (input, _) = space0(input)?;
    let (input, name) = parse_label(false)(input)?;
    
    if ! FINAL_DIRECTIVE.iter()
        .find(|&&a| a.to_lowercase() == name.to_lowercase())
        .is_some(){
            return Err(Err::Error(VerboseError::from_error_kind(
                input,
                ErrorKind::AlphaNumeric,
            )));
    }

    let (input, _) = space0(input)?;

    Ok((input, name))
}

/// Manage the call of a macro.
/// TODO use parse_forbidden_keyword
pub fn parse_macro_call(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    // BUG: added because of parsing issues. Need to find why and remove ot
    let (input, _) = space0(input)?;
    let (input, name) = parse_label(false)(input)?;

    // Check if the macro name is allowed
    if FIRST_DIRECTIVE.iter().chain(FINAL_DIRECTIVE.iter())
        .find(|&&a| a.to_lowercase() == name.to_lowercase())
        .is_some()
    {
        Err(Err::Failure(nom::error::ParseError::<&str>::from_error_kind(
            input,
            ErrorKind::AlphaNumeric,
        )))
    } else {
        let (input, args) = opt(alt((
            /*expr_list,  */ // initially a list of expression was used; now it is just plain strings
            separated_nonempty_list(
                tuple((tag(","), space0)),
                take_till(|c| c == ',' || c == '\n'),
            ),
            map(tag_no_case("(void)"), { |_| Vec::new() }),
        )))(input)?;

        Ok((
            input,
            Token::MacroCall(
                name,
                args.unwrap_or_default()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            ),
        ))
    }
}

fn parse_instr(name: &str) -> impl Fn(&str) -> IResult<&str, (), VerboseError<&str>> + '_ {
    move |input: &str| map(tuple((tag_no_case(name), not(alpha1), space0)), |_| ())(input)
}

/// ...
pub fn parse_djnz(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    map(preceded(parse_instr("DJNZ"), parse_expr), |expr| {
        Token::OpCode(Mnemonic::Djnz, Some(expr), None)
    })(input)
}

/// ...
pub fn expr_list(input: &str) -> IResult<&str, Vec<Expr>, VerboseError<&str>> {
    separated_nonempty_list(tuple((tag(","), space0)), alt((expr, string_expr)))(input)
}

/// ...
pub fn parse_assert(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, expr) = preceded(parse_instr("ASSERT"), expr)(input)?;

    let (input, comment) = opt(preceded(
        delimited(space0, tag(","), space0),
        delimited(tag("\""), take_until("\""), tag("\"")),
    ))(input)?;

    Ok((input, Token::Assert(expr, comment.map(|s| s.to_string()))))
}

/// ...
pub fn parse_align(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    map(preceded(parse_instr("ALIGN"), expr), |expr| {
        Token::Align(expr, None)
    })(input)
}

/// ...
pub fn parse_print(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    map(
        preceded(
            parse_instr("PRINT"),
            cut(separated_nonempty_list(
                delimited(space0, char(','), space0),
                alt((
                    formatted_expr,
                    map(expr, FormattedExpr::from),
                    map(string_between_quotes, {
                        |s: &str| FormattedExpr::from(Expr::String(s.to_string()))
                    }),
                )),
            )),
        ),
        |exps| Token::Print(exps),
    )(input)
}

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
fn formatted_expr(input: &str) -> IResult<&str, FormattedExpr, VerboseError<&str>> {
    let (input, _) = char('{')(input)?;
    let (input, format) = alt((
        value(ExprFormat::Int, tag_no_case("INT")),
        value(ExprFormat::Hex(Some(2)), tag_no_case("HEX4")),
        value(ExprFormat::Hex(Some(4)), tag_no_case("HEX8")),
        value(ExprFormat::Hex(Some(8)), tag_no_case("HEX2")),
        value(ExprFormat::Hex(None), tag_no_case("HEX")),
        value(ExprFormat::Bin(Some(8)), tag_no_case("BIN8")),
        value(ExprFormat::Bin(Some(16)), tag_no_case("BIN16")),
        value(ExprFormat::Bin(Some(32)), tag_no_case("BIN32")),
        value(ExprFormat::Bin(None), tag_no_case("BIN")),
    ))(input)?;
    let (input, _) = char('}')(input)?;

    let (input, _) = space0(input)?;

    let (input, exp) = expr(input)?;

    Ok((input, FormattedExpr::Formatted(format, exp)))
}

fn parse_comma(input: &str) -> IResult<&str, (), VerboseError<&str>> {
    map(tuple((space0, tag(","), space0)), |_| ())(input)
}

/// ...
pub fn parse_protect(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, start) = preceded(parse_instr("PROTECT"), expr)(input)?;

    let (input, end) = preceded(parse_comma, expr)(input)?;

    Ok((input, Token::Protect(start, end)))
}

/// ...
pub fn parse_logical_operator(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, operator) = alt((
        value(Mnemonic::And, parse_instr("AND")),
        value(Mnemonic::Or, parse_instr("Or")),
        value(Mnemonic::Xor, parse_instr("Xor")),
    ))(input)?;

    let (input, operand) = alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr,
    ))(input)?;

    Ok((input, Token::OpCode(operator, Some(operand), None)))
}

/// ...
pub fn parse_add_or_adc(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    alt((parse_add_or_adc_complete, parse_add_or_adc_shorten))(input)
}

/// Substraction with A register
pub fn parse_sub(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tag_no_case("SUB")(input)?;
    let (input, _) = space1(input)?;
    let (input, operand) = alt((
        parse_register8,
        parse_indexregister8,
        parse_hl_address,
        parse_indexregister_with_index,
        parse_expr,
    ))(input)?;

    Ok((input, Token::OpCode(Mnemonic::Sub, Some(operand), None)))
}

/// Par se the SBC instruction
pub fn parse_sbc(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tag_no_case("SBC")(input)?;
    let (input, _) = space1(input)?;
    let (input, opera) = alt((parse_register_a, parse_register_hl))(input)?;
    let (input, _) = parse_comma(input)?;

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
        Token::OpCode(Mnemonic::Sbc, Some(opera), Some(operb)),
    ))
}

/// Parse ADC and ADD instructions
pub fn parse_add_or_adc_complete(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
        return Err(nom::Err::Error(VerboseError::from_error_kind(input, ErrorKind::Alt)));
    }?;

    Ok((input, Token::OpCode(add_or_adc, Some(first), Some(second))))
}

/// TODO Find a way to not duplicate code with complete version
pub fn parse_add_or_adc_shorten(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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
        Token::OpCode(
            add_or_adc,
            Some(DataAccess::Register8(Register8::A)),
            Some(second),
        ),
    ))
}

/// ...
pub fn parse_push_n_pop(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, push_or_pop) = alt((
        value(Mnemonic::Push, parse_instr("PUSH")),
        value(Mnemonic::Pop, parse_instr("POP")),
    ))(input)?;

    let (input, register) = alt((parse_register16, parse_indexregister16))(input)?;

    Ok((input, Token::OpCode(push_or_pop, Some(register), None)))
}

/// ...
pub fn parse_ret(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    map(
        preceded(tag_no_case("RET"), opt(preceded(space1, parse_flag_test))),
        |cond| {
            Token::OpCode(
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
pub fn parse_inc_dec(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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

    Ok((input, Token::OpCode(inc_or_dec, Some(register), None)))
}

/// TODO manage other out formats
pub fn parse_out(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = parse_instr("OUT")(input)?;

    // get the port proposal
    let (input, port) = alt((parse_portc, parse_address))(input)?;

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

    Ok((input, Token::OpCode(Mnemonic::Out, Some(port), Some(value))))
}

/// Parse all the in flavors
pub fn parse_in(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = parse_instr("IN")(input)?;

    // get the port proposal
    let (input, destination) = parse_register8(input)?;
    let (input, _) = parse_comma(input)?;
    let (input, port) = alt((
        verify(parse_address, |_| {
            destination.get_register8().unwrap().is_a()
        }),
        parse_portc,
    ))(input)?;

    Ok((
        input,
        Token::OpCode(Mnemonic::In, Some(destination), Some(port)),
    ))
}

/// Parse the rst instruction
pub fn parse_rst(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = parse_instr("RST")(input)?;
    let (input, val) = parse_expr(input)?;

    Ok((input, Token::OpCode(Mnemonic::Rst, Some(val), None)))
}

/// Parse the IM instruction
pub fn parse_im(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = parse_instr("IM")(input)?;
    let (input, val) = parse_expr(input)?;

    Ok((input, Token::OpCode(Mnemonic::Im, Some(val), None)))
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
pub fn parse_shifts_and_rotations(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, oper) = alt((
        value(Mnemonic::Rlc, parse_instr("RLC")),
        value(Mnemonic::Rrc, parse_instr("RRC")),
        value(Mnemonic::Rl, parse_instr("RL")),
        value(Mnemonic::Rr, parse_instr("RR")),
        value(Mnemonic::Sla, parse_instr("SLA")),
        value(Mnemonic::Sra, parse_instr("SRA")),
        value(Mnemonic::Srl, parse_instr("SRL")),
        value(Mnemonic::Sll, parse_instr("SLL")),
    ))(input)?;

    let (input, arg) = alt((
        parse_register8,
        parse_hl_address,
        parse_indexregister_with_index,
    ))(input)?;

    Ok((input, Token::OpCode(oper, Some(arg), None)))
}

/// TODO reduce the flag space for jr"],
pub fn parse_call_jp_or_jr(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, call_jp_or_jr) = alt((
        value(Mnemonic::Jp, parse_instr("JP")),
        value(Mnemonic::Jr, parse_instr("JR")),
        value(Mnemonic::Call, parse_instr("CALL")),
    ))(input)?;

    let (input, flag_test) = opt(terminated(
        parse_flag_test,
        delimited(space0, tag(","), space0),
    ))(input)?;

    let (input, dst) = alt((
        verify(alt((parse_hl_address, parse_indexregister_address)), |_| {
            call_jp_or_jr.is_jp() && flag_test.is_none()
        }), // not possible for call and for jp/jr when there is flag
        parse_expr,
    ))(input)?;

    let flag_test = if flag_test.is_some() {
        Some(DataAccess::FlagTest(flag_test.unwrap()))
    } else {
        None
    };

    Ok((input, Token::OpCode(call_jp_or_jr, flag_test, Some(dst))))
}

/// ...
pub fn parse_flag_test(input: &str) -> IResult<&str, FlagTest, VerboseError<&str>> {
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
pub fn parse_register16(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    alt((
        parse_register_hl,
        parse_register_bc,
        parse_register_de,
        parse_register_af,
    ))(input)
}

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
pub fn parse_register8(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    alt((
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
pub fn parse_register_i(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    value(
        DataAccess::SpecialRegisterI,
        tuple((tag_no_case("I"), not(alphanumeric1))),
    )(input)
}

/// Parse register r
pub fn parse_register_r(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    value(
        DataAccess::SpecialRegisterR,
        tuple((tag_no_case("R"), not(alphanumeric1))),
    )(input)
}

macro_rules! parse_any_register8 {
    ($name: ident, $char:expr, $reg:expr) => {
        /// Parse register $char
        pub fn $name(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
            value(
                DataAccess::Register8($reg),
                tuple((tag_no_case($char), not(alphanumeric1))),
            )(input)
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
fn register16_parser<'a>(
    representation: &'static str,
    register: Register16,
) -> impl Fn(&'a str) -> IResult<&'a str, DataAccess, VerboseError<&str>> {
    move |input: &'a str| {
        value(
            DataAccess::Register16(register),
            tuple((tag_no_case(representation), not(alphanumeric1))),
        )(input)
    }
}

macro_rules! parse_any_register16 {
    ($name: ident, $char:expr, $reg:expr) => {
        /// Parse the $char register and return it as a DataAccess
        pub fn $name(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
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
pub fn parse_register_ix(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    value(
        DataAccess::IndexRegister16(IndexRegister16::Ix),
        tuple((tag_no_case("IX"), not(alphanumeric1))),
    )(input)
}

/// Parse the IY register
pub fn parse_register_iy(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    value(
        DataAccess::IndexRegister16(IndexRegister16::Iy),
        tuple((tag_no_case("IY"), not(alphanumeric1))),
    )(input)
}

// TODO find a way to not use that
macro_rules! parse_any_indexregister8 {
    ($($reg:ident, $alias:ident)*) => {$(
        paste::item_with_macros! {
            /// Parse register $reg
            pub fn [<parse_register_ $reg:lower>] (input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
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
pub fn parse_indexregister8(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    alt((
        parse_register_ixh,
        parse_register_iyh,
        parse_register_ixl,
        parse_register_iyl,
    ))(input)
}

/// Parse a 16 bits indexed register
pub fn parse_indexregister16(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    terminated(
        map(
            alt((
                map(tag_no_case("IX"), { |_| IndexRegister16::Ix }),
                map(tag_no_case("IY"), { |_| IndexRegister16::Iy }),
            )),
            |reg| DataAccess::IndexRegister16(reg),
        ),
        not(alphanumeric1),
    )(input)
}

/// Parse the use of an indexed register as (IX + 5)"
pub fn parse_indexregister_with_index(
    input: &str,
) -> IResult<&str, DataAccess, VerboseError<&str>> {
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
pub fn parse_portc(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    value(
        DataAccess::PortC,
        tuple((tag("("), space0, parse_register_c, space0, tag(")"))),
    )(input)
}

/// Parse an address access `(expression)`
pub fn parse_address(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    map(
        delimited(tag("("), expr, preceded(space0, tag(")"))),
        |address| DataAccess::Memory(address),
    )(input)
}

/// Parse (R16)
pub fn parse_reg_address(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
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
pub fn parse_hl_address(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
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
pub fn parse_indexregister_address(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
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
pub fn parse_expr(input: &str) -> IResult<&str, DataAccess, VerboseError<&str>> {
    let (input, expr) = expr(input)?;
    Ok((input, DataAccess::Expression(expr)))
}

/// Parse standard org directive
pub fn parse_org(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tuple((tag_no_case("ORG"), space1))(input)?;

    let (input, val) = expr(input)?;

    Ok((input, Token::Org(val, None)))
}

/// Parse defs instruction. TODO add optional parameters
pub fn parse_defs(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = tuple((tag_no_case("DEFS"), space1))(input)?;

    let (input, val) = expr(input)?;

    Ok((input, Token::Defs(val, None)))
}

/// Parse any opcode having no argument
pub fn parse_opcode_no_arg(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    alt((
        parse_opcode_no_arg1,
        parse_opcode_no_arg2,
        parse_opcode_no_arg3,
    ))(input)
}

fn parse_opcode_no_arg1(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, mnemonic) = alt((
        map(parse_instr("DI"), { |_| Mnemonic::Di }),
        map(parse_instr("CCF"), { |_| Mnemonic::Ccf }),
        map(parse_instr("EI"), { |_| Mnemonic::Ei }),
        map(parse_instr("EXX"), { |_| Mnemonic::Exx }),
        map(parse_instr("HALT"), { |_| Mnemonic::Halt }),
        map(parse_instr("LDIR"), { |_| Mnemonic::Ldir }),
        map(parse_instr("LDDR"), { |_| Mnemonic::Lddr }),
        map(parse_instr("LDI"), { |_| Mnemonic::Ldi }),
        map(parse_instr("LDD"), { |_| Mnemonic::Ldd }),
        map(parse_instr("NOPS2"), { |_| Mnemonic::Nops2 }),
        map(parse_instr("NOP"), { |_| Mnemonic::Nop }),
        map(parse_instr("OUTD"), { |_| Mnemonic::Outd }),
        map(parse_instr("OUTI"), { |_| Mnemonic::Outi }),
    ))(input)?;

    Ok((input, Token::OpCode(mnemonic, None, None)))
}

fn parse_opcode_no_arg2(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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

    Ok((input, Token::OpCode(mnemonic, None, None)))
}

fn parse_opcode_no_arg3(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, mnemonic) = alt((
        value(Mnemonic::Daa, parse_instr("DAA")),
        value(Mnemonic::Neg, parse_instr("NEG")),
        value(Mnemonic::Otdr, parse_instr("OTDR")),
        value(Mnemonic::Otir, parse_instr("OTIR")),
        value(Mnemonic::Rld, parse_instr("RLD")),
        value(Mnemonic::Rrd, parse_instr("RRD")),
    ))(input)?;

    Ok((input, Token::OpCode(mnemonic, None, None)))
}

fn parse_snaset(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    let (input, _) = parse_instr("SNASET")(input)?;
    let (input, flag) = parse_flag(input)?;
    let (input, _) = space1(input)?;
    let (input, value) = parse_flag_value(input)?;
    let (input, _) = space0(input)?;

    Ok((input, Token::SnaSet(flag, value)))
}

/// Parse a comment that start by `;` and ends at the end of the line.
pub fn comment(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    map(
        preceded(tag(";"), take_till(|ch| ch == '\n')),
        |string: &str| Token::Comment(string.iter_elements().collect::<String>()),
    )(input)
}

/// Usefull later for db
pub fn string_between_quotes(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    delimited(char('\"'), is_not("\""), char('\"'))(input)
}

/// TODO
pub fn string_expr(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    map(string_between_quotes, |string| {
        Expr::String(string.to_string())
    })(input)
}

/// Parse a label(label: S)
pub fn parse_label(doubledots: bool) -> impl Fn(&str) -> IResult<&str, String, VerboseError<&str>> {
    move |input: &str| {
        // Get the label

        let (input, first) =
            one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ._")(input)?;
        let (input, middle) =
            is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.")(input)?;

        let input = if doubledots {
            let (input, _) = opt(tag_no_case(":"))(input)?;
            input
        } else {
            input
        };

        let label = format!("{}{}", first, middle.iter_elements().collect::<String>());

        let impossible = ["af", "hl", "de", "bc", "ix", "iy", "ixl", "ixh"];
        if impossible.iter().any(|val| val == &label.to_lowercase()) {
            Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
        } else {
            Ok((input, label))
        }
    }
}

pub fn prefixed_label_expr(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    let (input, prefix) = alt((
        value(LabelPrefix::Bank, tag_no_case("{bank}")),
        value(LabelPrefix::Page, tag_no_case("{page}")),
        value(LabelPrefix::Pageset, tag_no_case("{pageset}")),
    ))(input)?;
    let (input, label) = preceded(
        space0,
        alt((
            parse_label(false),
            map(tag_no_case("$"), |s: &str| s.to_string()),
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
pub fn parse_value(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    let (input, val) = alt((hex_number, dec_number, bin_u16))(input)?;
    Ok((input, Expr::Value(val as i32)))
}

/// Read a parenthesed expression
pub fn parens(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    delimited(
        delimited(space0, tag("("), space0),
        map(map(expr, Box::new), Expr::Paren),
        delimited(space0, tag(")"), space0),
    )(input)
}

/// Get a factor
pub fn factor(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    context(
        "factor",
        delimited(
            space0,
            context(
                "alt",
                alt((
                    // Manage functions
                    parse_unary_functions,
                    parse_binary_functions,
                    parse_duration,
                    parse_assemble,
                    // manage values
                    map(alt((hex_number, bin_u16, dec_number)), |d: u16| {
                        Expr::Value(d as i32)
                    }),
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
    )(input)
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
pub fn term<'a>(input: &'a str) -> IResult<&'a str, Expr, VerboseError<&str>> {
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
fn parse_oper<'a, F>(
    inner: F,
    pattern: &'static str,
    symbol: Oper,
) -> impl Fn(&'a str) -> IResult<&'a str, (Oper, Expr), VerboseError<&str>>
where
    F: Fn(&'a str) -> IResult<&'a str, Expr, VerboseError<&str>>,
{
    move |input: &'a str| {
        let (input, _) = space0(input)?;
        let (input, _) = tag_no_case(pattern)(input)?;
        let (input, _) = space0(input)?;
        let (input, operation) = inner(input)?;

        Ok((input, (symbol, operation)))
    }
}

/// Parse an expression
pub fn expr(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    let (input, initial) = comp(input)?;
    let (input, remainder) = many0(alt((
        parse_oper(comp, "<=", Oper::LowerOrEqual),
        parse_oper(comp, "<", Oper::StrictlyLower),
        parse_oper(comp, ">=", Oper::GreaterOrEqual),
        parse_oper(comp, ">", Oper::StrictlyGreater),
        parse_oper(comp, "==", Oper::Equal),
        parse_oper(comp, "!=", Oper::Different),
    )))(input)?;

    Ok((input, fold_exprs(initial, remainder)))
}

/// parse functions with one argument
pub fn parse_unary_functions(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
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
pub fn parse_binary_functions(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
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
) -> impl Fn(&'a str) -> IResult<&str, Token, VerboseError<&str>> {
    move |input: &'a str| {
        let (input, _) = tuple((tag_no_case(function_name), space0, char('('), space0))(input)?;

        let (input, token) = parse_token(input)?;

        let (input, _) = tuple((space0, tag(")")))(input)?;

        Ok((input, token))
    }
}

/// Parse the duration function
pub fn parse_duration(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    let (input, token) = token_function("duration")(input)?;
    Ok((input, Expr::Duration(Box::new(token))))
}

/// Parse the single opcode assembling function
pub fn parse_assemble(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
    let (input, token) = token_function("opcode")(input)?;
    Ok((input, Expr::OpCode(Box::new(token))))
}

/// Parse operation related to + - & |
pub fn comp(input: &str) -> IResult<&str, Expr, VerboseError<&str>> {
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
    use crate::preamble::*;

    #[test]
    fn parse_test_cond() {



        let res = inner_code(
            " nop
        endif"
        );
        assert!(res.is_ok());
        assert_eq!(res.unwrap().1.len(), 1);

        let res = inner_code(
            " nop
        else",
        );
        assert!(res.is_ok());
        assert_eq!(res.unwrap().1.len(), 1);


        let res = parse_conditional_condition(KindOfConditional::If)("THING");
        assert!(res.is_ok());

        let res = std::dbg!(parse_conditional(
            "if THING
        nop
        endif 
        ",
        ));
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0.trim());

        let res = std::dbg!(parse_conditional(
            "if THING
        nop
        endif ",
        ));
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0.trim());



        let res = parse_conditional(
            "if THING
        nop
        else
        nop
        endif",
        );
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0);

        let res = parse_conditional(
            "ifndef THING
        nop
        else
        nop
        endif",
        );
        assert!(res.is_ok());
        assert_eq!("", res.unwrap().0);

        let res = std::dbg!(parse_conditional("if demo_system_music_activated != 0
        ; XXX Ensure memory is properly set
        ld bc, 0x7fc2 : out (c), c
        jp PLY_AKYst_Play
    else
        WAIT_CYCLES 64*16
        ret
    endif"));
    assert!(res.is_ok());
    assert_eq!("", res.unwrap().0);

    }

    #[test]
    fn parse_indexregister8() {
        assert_eq!(
            parse_register_ixl("ixl"),
            Ok(("", DataAccess::IndexRegister8(IndexRegister8::Ixl)))
        );

        assert_eq!(
            parse_register_ixl("lx"),
            Ok(("", DataAccess::IndexRegister8(IndexRegister8::Ixl)))
        );

        assert!(parse_register_iyl("ixl").is_err());
    }

    #[test]
    fn test_parse_expr_format() {
        assert_eq!(
            formatted_expr("{hex} VAL"),
            Ok((
                "",
                FormattedExpr::Formatted(ExprFormat::Hex(None), Expr::Label("VAL".to_string()))
            ))
        );
    }

    #[test]
    fn test_parse_print() {
        assert_eq!(
            Ok((
                "",
                Token::Print(vec![FormattedExpr::Raw(Expr::Label("VAR".to_string()))])
            )),
            parse_print("PRINT VAR")
        );

        assert_eq!(
            Ok((
                "",
                Token::Print(vec![FormattedExpr::Formatted(
                    ExprFormat::Hex(None),
                    Expr::Label("VAR".to_string())
                )])
            )),
            parse_print("PRINT {hex}VAR")
        );

        assert_eq!(
            Ok((
                "",
                Token::Print(vec![FormattedExpr::Raw(Expr::String("hello".to_string()))])
            )),
            parse_print("PRINT \"hello\"")
        );
    }


    #[test]
    fn test_standard_repeat() {
        let z80 = std::dbg!("  repeat 5
        nop
            endrepeat
            ");
        let res = parse_repeat(z80);
        assert!(res.is_ok(), "{:?}", res);
        let res = res.unwrap();
        assert_eq!(res.0.trim().len(), 0, "{:?}", res);
    }
}
