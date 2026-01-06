use core::str;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;

use cpclib_common::smallvec::SmallVec;
use cpclib_common::winnow::ascii::{Caseless, alpha1, line_ending, newline, space0};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, eof, not, opt, peek, preceded, terminated
};
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
pub use super::registers::{
    parse_indexregister_with_index, parse_indexregister8, parse_indexregister16, parse_register_i,
    parse_register_ix, parse_register_iy, parse_register_r, parse_register8, parse_register16
};
use super::*;
use crate::hashed_choice;
use crate::parser::parser::{DOTTED_END_DIRECTIVE, END_DIRECTIVE};
use crate::preamble::*;

// Compile-time, case-insensitive FNV-1a hash for ASCII
pub(crate) const fn fnv1a_ascii_upper(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF29CE484222325u64;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let upper = if b >= b'a' && b <= b'z' { b - 32 } else { b };
        hash ^= upper as u64;
        hash = hash.wrapping_mul(0x100000001B3);
        i += 1;
    }
    hash
}

#[inline(always)]
pub(crate) fn eq_ascii_nocase(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| x.eq_ignore_ascii_case(&y))
}

#[derive(PartialEq)]
pub enum SaveKind {
    Save,
    WriteDirect
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

    // Apply the right parsing using hash-based matching for performance and consistency
    let h = fnv1a_ascii_upper(word);
    let token: LocatedTokenInner = match h {
        h if hashed_choice!(h, word, b"LD") => parse_ld(true).parse_next(input),
        h if hashed_choice!(h, word, b"ADC") => parse_add_or_adc(Mnemonic::Adc).parse_next(input),
        h if hashed_choice!(h, word, b"ADD") => parse_add_or_adc(Mnemonic::Add).parse_next(input),
        h if hashed_choice!(h, word, b"AND") => parse_logical_operator(Mnemonic::And).parse_next(input),

        h if hashed_choice!(h, word, b"BIT") => parse_res_set_bit(Mnemonic::Bit).parse_next(input),

        h if hashed_choice!(h, word, b"CALL") => parse_call_jp_or_jr(Mnemonic::Call).parse_next(input),
        h if hashed_choice!(h, word, b"CP") => parse_cp.parse_next(input),

        h if hashed_choice!(h, word, b"DEC") => parse_inc_dec(Mnemonic::Dec).parse_next(input),
        h if hashed_choice!(h, word, b"DJNZ") => parse_djnz.parse_next(input),

        h if hashed_choice!(h, word, b"EX") => {
            alt((parse_ex_af, parse_ex_hl_de, parse_ex_mem_sp)).parse_next(input)
        },

        h if hashed_choice!(h, word, b"EXA") => Ok(LocatedTokenInner::new_opcode(Mnemonic::ExAf, None, None)),
        h if hashed_choice!(h, word, b"EXD") => Ok(LocatedTokenInner::new_opcode(Mnemonic::ExHlDe, None, None)),

        h if hashed_choice!(h, word, b"IN") => parse_in.parse_next(input),
        h if hashed_choice!(h, word, b"INC") => parse_inc_dec(Mnemonic::Inc).parse_next(input),
        h if hashed_choice!(h, word, b"IM") => parse_im.parse_next(input),

        h if hashed_choice!(h, word, b"JP") => parse_call_jp_or_jr(Mnemonic::Jp).parse_next(input),
        h if hashed_choice!(h, word, b"JR") => parse_call_jp_or_jr(Mnemonic::Jr).parse_next(input),

        h if hashed_choice!(h, word, b"OR") => parse_logical_operator(Mnemonic::Or).parse_next(input),
        h if hashed_choice!(h, word, b"OUT") => parse_out.parse_next(input),

        h if hashed_choice!(h, word, b"POP") => parse_push_n_pop(Mnemonic::Pop).parse_next(input),
        h if hashed_choice!(h, word, b"PUSH") => parse_push_n_pop(Mnemonic::Push).parse_next(input),

        h if hashed_choice!(h, word, b"RES") => parse_res_set_bit(Mnemonic::Res).parse_next(input),
        h if hashed_choice!(h, word, b"RET") => parse_ret.parse_next(input),
        h if hashed_choice!(h, word, b"RLC") => alt((
            parse_shifts_and_rotations(Mnemonic::Rlc),
            parse_shifts_and_rotations_fake(Mnemonic::Rlc)
        )).parse_next(input),
        h if hashed_choice!(h, word, b"RL") => alt((
            parse_shifts_and_rotations(Mnemonic::Rl),
            parse_shifts_and_rotations_fake(Mnemonic::Rl)
        )).parse_next(input),
        h if hashed_choice!(h, word, b"RRC") => alt((
            parse_shifts_and_rotations(Mnemonic::Rrc),
            parse_shifts_and_rotations_fake(Mnemonic::Rrc)
        )).parse_next(input),
        h if hashed_choice!(h, word, b"RR") => alt((
            parse_shifts_and_rotations(Mnemonic::Rr),
            parse_shifts_and_rotations_fake(Mnemonic::Rr),
        )).parse_next(input),
        h if hashed_choice!(h, word, b"RST") => {
                alt((
                parse_rst_fake, parse_rst
            )).parse_next(input)
        },

        h if hashed_choice!(h, word, b"SBC") => parse_sbc.parse_next(input),
        h if hashed_choice!(h, word, b"SET") => parse_res_set_bit(Mnemonic::Set).parse_next(input),
        h if hashed_choice!(h, word, b"SL") /*1*/  => cut_err(preceded(('1', my_space1), parse_shifts_and_rotations(Mnemonic::Sl1))).parse_next(input),
        h if hashed_choice!(h, word, b"SLA") => alt((
            parse_shifts_and_rotations(Mnemonic::Sla),
            parse_shifts_and_rotations_fake(Mnemonic::Sla),
        )).parse_next(input),
        h if hashed_choice!(h, word, b"SLL") => alt((
            parse_shifts_and_rotations(Mnemonic::Sl1),
            parse_shifts_and_rotations_fake(Mnemonic::Sl1),
        )).parse_next(input),
        h if hashed_choice!(h, word, b"SRA") => alt((
            parse_shifts_and_rotations(Mnemonic::Sra),
            parse_shifts_and_rotations_fake(Mnemonic::Sra)
        )).parse_next(input),
        h if hashed_choice!(h, word, b"SRL") => alt((
            parse_shifts_and_rotations(Mnemonic::Srl),
            parse_shifts_and_rotations_fake(Mnemonic::Srl),
        )).parse_next(input),
        h if hashed_choice!(h, word, b"SUB") => parse_sub.parse_next(input),

        h if hashed_choice!(h, word, b"XOR") => parse_logical_operator(Mnemonic::Xor).parse_next(input),

        _ => {
            Err(ErrMode::Backtrack(Z80ParserError::from_input(
                input
            )))
        },
    }?;

    let token = token.into_located_token_between(&input_start, *input);
    Ok(token)
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

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn build_span_covering(span: &Z80Span, right: &Z80Span) -> InnerZ80Span {
    // Safety: We assume both spans are from the same buffer/context.
    let left = &span.0;
    let right = &right.0;
    debug_assert!(
        left.state == right.state,
        "Spans must have the same context"
    );

    // If either is empty, return the other
    if left.is_empty() {
        return *right;
    }
    if right.is_empty() {
        return *left;
    }

    let left_bytes = left.as_bstr();
    let right_bytes = right.as_bstr();

    let start_ptr = left_bytes.as_ptr();
    let end_ptr = unsafe { right_bytes.as_ptr().add(right_bytes.len()) };
    let total_len = (end_ptr as usize).wrapping_sub(start_ptr as usize);

    // Safety: start_ptr and total_len are valid and within the same buffer
    let covering_bytes = unsafe { std::slice::from_raw_parts(start_ptr, total_len) };
    left.update_slice(covering_bytes)
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
    F: Parser<InnerZ80Span, O, ErrMode<E>>,
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
    F: Parser<InnerZ80Span, O, ErrMode<E>>,
    G: Parser<InnerZ80Span, P, ErrMode<E>>,
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
                            return Err(ErrMode::Backtrack(e.append(i, &start_i)));
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

            alt((eof, line_ending))
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

pub fn my_repeat1<I, O, C, E, F>(mut f: F) -> impl Parser<I, C, ErrMode<E>>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, ErrMode<E>>,
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
    F: Parser<I, O, ErrMode<E>>,
    E: ParserError<I>
{
    let start = i.checkpoint();
    match f.parse_next(i) {
        Err(e) => Err(e.append(i, &start)),
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
