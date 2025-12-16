// Registers module - contains register parsing functions and constants

use choice_nocase::choice_nocase;
use cpclib_common::winnow::ascii::{Caseless, alpha1, alphanumeric1};
use cpclib_common::winnow::combinator::{alt, not, preceded, terminated, opt};
use cpclib_common::winnow::error::{ErrMode, ParserError};
use cpclib_common::winnow::stream::{Stream, UpdateSlice};
use cpclib_common::winnow::token::{one_of, take};
use cpclib_common::winnow::{ModalResult, Parser};
use cpclib_tokens::{BinaryOperation, DataAccessElem, IndexRegister8, IndexRegister16, Register8, Register16};

use super::error::*;
use super::obtained::*;
use super::*;

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
    }

    alt((
        (
            parse_register16,
            preceded(
                b'.',
                alt((
                    Caseless("low").map(|_| Reg16Modifier::Low),
                    Caseless("high").map(|_| Reg16Modifier::High)
                ))
            ),
            my_space0
        )
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
    let da = (Caseless("I"), not(alphanumeric1))
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
    let da = (Caseless("R"), not(alphanumeric1))
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
        let span = (
            Caseless(representation),
            not(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_')))
        )
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

/// Parse any indexed 8-bit register (IXH, IXL, IYH, IYL)
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

/// Parse a 16-bit indexed register (IX or IY)
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

/// Parse an indexed register with optional +/- offset, e.g. (IX+5)
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_indexregister_with_index(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedDataAccess, Z80ParserError> {
    let start_checkpoint = input.checkpoint();
    let start_eof_offset = input.eof_offset();
    let (open, _, reg) = (alt((b'(', b'[')), my_space0, parse_indexregister16).parse_next(input)?;

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
