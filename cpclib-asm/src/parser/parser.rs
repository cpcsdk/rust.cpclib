
#![allow(clippy::cast_lossless)]

use core::str;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};

use choice_nocase::choice_nocase;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use cpclib_common::smallvec::SmallVec;
use cpclib_common::winnow::ascii::{Caseless, alpha1, line_ending, newline, space0};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, eof, not, opt, peek, preceded, terminated
};
#[allow(deprecated)]
use cpclib_common::winnow::error::ErrorKind;
use cpclib_common::winnow::error::{AddContext, ErrMode, ParserError, StrContext};
use cpclib_common::winnow::stream::{Accumulate, AsBStr, AsChar, Stream, UpdateSlice};
use cpclib_common::winnow::token::{one_of, take_till, take_until, take_while};
use cpclib_common::winnow::{ModalResult, Parser};
// use crc::*;
use obtained::LocatedTokenInner;

use super::context::*;
use super::error::*;
use super::expression::*;
use super::instructions::*;
use super::obtained::*;
use super::orgams::*;
use super::*;
use crate::preamble::*;

include!(concat!(
    env!("OUT_DIR"),
    "/basm_directives_name_generated.rs"
));

// const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

trait AccumulateSeveral<O>: Accumulate<O> {
    fn accumulate_several(&mut self, items: &mut Vec<O>);
}
#[cfg(test)]
mod parse_factor_robust_suite {
    use crate::test::parse_test;

    

    use super::*;
    #[test]
    fn test_parse_factor_robust() {
        // Numbers
        assert!(parse_test(parse_factor, "42").is_ok());
        assert!(parse_test(parse_factor, "0x2A").is_ok());
        assert!(parse_test(parse_factor, "$2A").is_ok());
        assert!(parse_test(parse_factor, "%101010").is_ok());
        assert!(parse_test(parse_factor, "'A'").is_ok());
        assert!(parse_test(parse_factor, "0b1010").is_ok());
        // Labels
        assert!(parse_test(parse_factor, "LABEL").is_ok());
        assert!(parse_test(parse_factor, "BREAKPOINT_METHOD").is_ok());
        assert!(parse_test(parse_factor, "BREAKPOINT_WITH_WINAPE_BYTES").is_ok());
        assert!(parse_test(parse_factor, "BREAKPOINT_WITH_SNAPSHOT_MODIFICATION").is_ok());
        assert!(parse_test(parse_factor, "LABEL").is_ok());
        assert!(parse_test(parse_factor, "_label").is_ok());
        assert!(parse_test(parse_factor, "label123").is_ok());
        assert!(parse_test(parse_factor, "@bad").is_ok());
        // Function calls
        assert!(parse_test(parse_factor, "func()").is_ok());
        assert!(parse_test(parse_factor, "func(1,2,3)").is_ok());
        assert!(parse_test(parse_factor, "func(label, 42)").is_ok());
        assert!(parse_test(parse_factor, "f(1)").is_ok());
        // Parenthesized expressions
        assert!(parse_test(parse_factor, "(1)").is_ok());
        assert!(parse_test(parse_factor, "(LABEL)").is_ok());
        assert!(parse_test(parse_factor, "(1+2)").is_ok());
        // Strings
        assert!(parse_test(parse_factor, "\"hello\"").is_ok());
        assert!(parse_test(parse_factor, "'c'").is_ok());
        // Unary operations
        assert!(parse_test(parse_factor, "-42").is_ok());
        assert!(parse_test(parse_factor, "+42").is_ok());
        assert!(parse_test(parse_factor, "~42").is_ok());
        // Nested function calls
        assert!(parse_test(parse_factor, "f(g(1),h(2,3))").is_ok());
        // Error cases
        assert!(parse_test(parse_factor, "").is_err());
        assert!(parse_test(parse_factor, "(").is_err());
        assert!(parse_test(parse_factor, "func(1,2").is_err());
        assert!(parse_test(parse_factor, "1 2").is_err());
        // Complex expressions (should succeed as factor if valid)
        assert!(parse_test(parse_factor, "(func(1)+2)").is_ok());
        assert!(parse_test(parse_factor, "((42))").is_ok());
        // Large number
        assert!(parse_test(parse_factor, "123456789").is_ok());
        // Hex with prefix
        assert!(parse_test(parse_factor, "0XDEADBEEF").is_ok());
        // Binary with prefix
        assert!(parse_test(parse_factor, "0b1101").is_ok());
        // Label with dot
        assert!(parse_test(parse_factor, "label.with.dot").is_ok());
        // Label with braces (we do not yet handle that. But we'll have too later)
        //assert!(parse_test(parse_factor, "label{macro}").is_ok());
    }
}

impl<O> AccumulateSeveral<O> for Vec<O> {
    fn accumulate_several(&mut self, items: &mut Vec<O>) {
        self.append(items);
    }
}

// TODO search why they are listed to forbid label naming. Delete it if unneeded
pub const REGISTERS: &[&[u8]] = &[b"AF", b"HL", b"DE", b"BC", b"IX", b"IY", b"IXL", b"IXH"];

// INSTRUCTIONS constant moved to instructions module (re-exported via mod.rs)


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
            .map(|l| Arc::<LocatedListing>::try_unwrap(l).expect("Arc should have single owner"))
    }
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

    let _before_let = input.checkpoint();
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

// parse_fname is now defined in expression module

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
// MIGRATED: parse_z80_directive_with_block -> directives.rs
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
        // use parse_forbidden_keyword to detect an END directive token
        let stop = opt(parse_forbidden_keyword).parse_next(input)?;
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

// MIGRATED: parse_string, parse_stringlike_without_quote, my_escaped -> expression.rs

// MIGRATED: parse_charset, parse_charset_start_stop_end, parse_charset_string -> directives.rs

/// Parser for the include directive

/// Parse the include directive

/// Parse for the various binary include directives

// MIGRATED: parse_incbin -> directives.rs

// MIGRATED: parse_write_direct_memory -> directives.rs

#[derive(PartialEq)]
pub enum SaveKind {
    Save,
    WriteDirect
}

/// Parse both save directive and write direct in a file
/// Parse the save directive

/// Parse  UNDEF directive.
// MIGRATED: parse_undef -> directives.rs

// MIGRATED: parse_section -> directives.rs

/// Parse the range directive

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

// MIGRATED: parse_struct_directive / parse_struct_directive_inner -> directives.rs

/// Parse any directive
/// Parse directive - main dispatcher
/// Migrated to directives.rs

/// Parse directive new - core directive parser with size-based routing
/// Migrated to directives.rs

/// All parse_directive_of_size_* helper functions
/// Migrated to directives.rs

// MIGRATED: parse_conditional / parse_conditional_condition -> directives.rs
// MIGRATED: KindOfConditional -> directives.rs
// MIGRATED: parse_conditional / parse_conditional_condition -> directives.rs

/// Parse a breakpoint instruction
/// Parse breakpoint directive and related helper functions
/// Migrated to directives.rs

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

/// Parse bankset directive
/// Migrated to directives.rs

/// Parse the buildsna directive

/// Parse the run directive

// MIGRATED: directive_with_expr macro and parse_map, parse_limit, parse_waitnops, parse_return -> directives.rs

// MIGRATED: parse_startingindex -> directives.rs
// MIGRATED: parse_assembler_control* -> directives.rs

// (migrated) parse_stable_ticker, parse_stable_ticker_start, parse_stable_ticker_stop live in directives.rs

/// Parse bank directive
/// Migrated to directives.rs

/// Parse skip directive
/// Migrated to directives.rs

/// Parse export directive and ExportKind enum
/// Migrated to directives.rs

// (migrated) parse_db_or_dw_or_str and DbDwStr live in directives.rs

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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
/// Consume the word and the empty space after
pub fn parse_word(
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

// MIGRATED: expr_list -> expression.rs

/// Parse the assert directive
/// Migrated to directives.rs

/// ...
/// Parse the align directive
/// Migrated to directives.rs

// (migrated) parse_print_inner lives in directives.rs

/// Parse the print directive
/// Migrated to directives.rs

/// Parse the fail directive  
/// Migrated to directives.rs

/// Parse formatted expression for print like directives
/// WARNING: only formated case is taken into account
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
// (migrated) formatted_expr lives in directives.rs

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
pub fn my_line_ending(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    alt((line_ending.take(), ':'.take()))
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_comma(input: &mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    delimited(my_space0, ','.take(), my_space0)
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

pub fn parse_comma_multiline(
    input: &mut InnerZ80Span
) -> ModalResult<InnerZ80Span, Z80ParserError> {
    let cloned = *input;
    (parse_comma, opt((newline, my_space0)))
        .take()
        .map(|s| cloned.update_slice(s))
        .parse_next(input)
}

/// Parse the protect directive
/// Migrated to directives.rs

/// ...
// XXX to remove as soon as possible
// named_attr!(#[doc="TODO"],
// parse_dollar <&str, Expr>, do_parse!(
// tag!("$") >>
// (Expr::Label(String::from("$")))
// )
// );

/// Parse any standard 16bits register
/// TODO rename to emphasize it is standard reigsters
// Register functions moved to registers.rs module
pub use super::registers::{
    parse_indexregister_with_index, parse_indexregister8, parse_indexregister16, parse_register_i,
    parse_register_ix, parse_register_iy, parse_register_r, parse_register8, parse_register16
};

// (migrated) parse_indexregister8/16/with_index live in registers.rs

// MIGRATED: parse_portc, parse_portnn -> instructions.rs

/// Parse standard org directive
/// Migrated to directives.rs

/// Parse defs instruction. TODO add optional parameters
/// Parse defs directive
/// Migrated to directives.rs

// parse_opcode_no_arg moved to instructions module (re-exported via mod.rs)

/// Parse snainit directive
/// Migrated to directives.rs

/// Parse snaset directive
/// Migrated to directives.rs

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

// MIGRATED: string_expr -> expression.rs

pub fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
    use std::ops::Deref;
    let ctx = Box::new(
        ParserContextBuilder::default()
            .set_context_name("TEST")
            .build(code)
    );
    let span = Z80Span::new_extra(code, ctx.deref());
    (ctx, span)
}

// (deprecated) parse_end_directive was used only in local tests; removed

// Test are deactivated, API is not enough stabilized and tests are broken
#[cfg(test)]
pub mod test {
    use std::ops::Deref;

    use cpclib_common::winnow::combinator::repeat;
    use cpclib_common::winnow::error::ParseError;

    use super::*;
    // stable ticker parsers were moved to directives.rs and re-exported via crate::parser
    use crate::parser::parse_stable_ticker_start;

    #[derive(Debug)]
    pub struct TestResult<O: std::fmt::Debug> {
        pub ctx: Box<ParserContext>,
        pub span: Z80Span,
        pub res: Result<O, ParseError<InnerZ80Span, Z80ParserError>>
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

    pub fn parse_test<O, P: Parser<InnerZ80Span, O, Z80ParserError>>(
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

    // removed: test_parse_end_directive (helper removed)

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
    fn test_string() {
        assert!(dbg!(parse_test(parse_string, "\"hello\"")).is_ok());
        assert!(
            dbg!(parse_test(
                parse_string,
                "\"Andy Severn - Lop Earsfap.BIN\""
            ))
            .is_ok()
        );
        assert!(
            dbg!(parse_test(
                parse_string,
                "\"ArkosTracker3\\\\Andy Severn - Lop Earsfap.BIN\""
            ))
            .is_ok()
        );
        assert!(
            dbg!(parse_test(
                parse_string,
                "\"datasets\\\\ArkosTracker3\\\\ArkosTracker3\\\\Andy Severn - Lop Earsfap.BIN\""
            ))
            .is_ok()
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
        let mut vec = Vec::with_capacity(8);
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
            parse_z80_line_complete(&mut Vec::new()),
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
            r#in.iter()
                .map(|t| t.to_token().into_owned())
                .collect::<Vec<_>>(),
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
        let mut tokens = Vec::with_capacity(16);

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
            let mut tokens = Vec::with_capacity(16);
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
            r#""fdfd\" et voila""#,
            r#""HE\"LL\nO\t""#
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
        let mut tokens = Vec::with_capacity(16);
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


    static EXPR2_CASES: &[(&str, bool)] = &[
            ("1<=2", true),
            ("A<=B", true),
            ("func(1,2)<=3", true),
            ("(1+2)<=3", true),
            ("func(1,2)<=func2(3,4)", true),

            ("1<2", true),
            ("A<B", true),
            ("func(1,2)<3", true),
            ("(1+2)<3", true),
            ("func(1,2)<func2(3,4)", true),

            ("1>2", true),
            ("A>B", true),
            ("func(1,2)>3", true),
            ("(1+2)>3", true),
            ("func(1,2)>func2(3,4)", true),
 
            ("1==2", true),
            ("A==B", true),
            ("func(1,2)==3", true),
            ("(1+2)==3", true),
            ("func(1,2)==func2(3,4)", true),

            ("1!=2", true),
            ("A!=B", true),
            ("func(1,2)!=3", true),
            ("(1+2)!=3", true),
            ("func(1,2)!=func2(3,4)", true),



            ("BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES", true),
            ("BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION", true),


            ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)", true),
            ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)", true),



        ];

    
    #[test]
    fn test_parse_expr2_robust() {
        for (input, should_succeed) in EXPR2_CASES.iter().chain(COMP_CASES.iter()) {
            let result = parse_test(expr2, input);
            if *should_succeed {
                assert!(result.is_ok(), "Should parse '{}', got {:?}", input, result);
            } else {
                assert!(result.is_err(), "Should fail to parse '{}', got {:?}", input, result);
            }
        }
    }

    static COMP_CASES: &[(&str, bool)] = &[
                      ("1+2", true),
            ("A+B", true),
            ("func(1,2)+3", true),
            ("(1+2)*3", true),
            ("1+2*3", true),
            ("(A)", true),
            ("A+B*C", true),
            ("A+B*C-D", true),
            ("A+B*C-D/E", true),
            ("func(1,2)+func2(3,4)", true),
            ("1+", false),
            ("+1", true),
            ("A+", false),
            ("(1+2", false),
            ("A+B*", false),
            ("func(1,2", false),
            ("A+B C", false),
        ];

    #[test]
    fn test_parse_comp_robust() {
        
        for (input, should_succeed) in COMP_CASES {
            let result = parse_test(comp, input);
                       if *should_succeed {
                assert!(result.is_ok(), "Should parse '{}', got {:?}", input, result);
            } else {
                assert!(result.is_err(), "Should fail to parse '{}', got {:?}", input, result);
            }
        }
    }

    static LOCATED_EXPR_CASES: &[(&str, bool)] = &[

            ("BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES", true),
            ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)", true),
            ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)||(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)", true),
            ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)||(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)", true),
            ("((BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES))", true),

    ];

    #[test]
    fn test_parse_located_expression_robust() {
        
        for (input, should_succeed) in COMP_CASES.iter().chain(EXPR2_CASES.iter()).chain(LOCATED_EXPR_CASES.iter()) {
            let result = parse_test(located_expr, input);
            if *should_succeed {
                if result.is_err() {
                    eprintln!("FAIL: Should parse '{}', got {:?}", input, result);
                }
                assert!(result.is_ok(), "Should parse '{}', got {:?}", input, result);
            } else {
                if result.is_ok() {
                    eprintln!("FAIL: Should fail to parse '{}', got {:?}", input, result);
                }
                assert!(result.is_err(), "Should fail to parse '{}', got {:?}", input, result);
            }
        }
    }
}
