// Expression module - contains expression parsing functions

use std::fmt::Debug;

use choice_nocase::choice_nocase;
use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use cpclib_common::winnow::ascii::{Caseless, alpha1, alphanumeric1, line_ending};
use cpclib_common::winnow::combinator::{
    alt, delimited, eof, not, opt, peek, preceded, repeat, separated, terminated
};
use cpclib_common::winnow::error::{AddContext, ErrMode, ParserError, StrContext};
use cpclib_common::winnow::stream::{AsBStr, AsBytes, AsChar, Stream, UpdateSlice};
use cpclib_common::winnow::token::{none_of, one_of, take_while};
use cpclib_common::winnow::{ModalResult, Parser};
use cpclib_sna::FlagValue;
use cpclib_tokens::{
    AssemblerFlavor, BinaryOperation, DataAccessElem, Expr, ExprElement, FlagTest, LabelPrefix,
    Register16, UnaryOperation, UnaryTokenOperation
};
use smallvec::SmallVec;

use super::error::*;
use super::instructions::INSTRUCTIONS;
use super::obtained::*;
use super::parser::{
    END_DIRECTIVE, REGISTERS, STAND_ALONE_DIRECTIVE, START_DIRECTIVE, parse_comma
};
use super::*;

// Include build-time generated forbidden names
include!(concat!(env!("OUT_DIR"), "/forbidden_names_generated.rs"));

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
pub fn parse_function_arguments(
    input: &mut InnerZ80Span
) -> ModalResult<Vec<LocatedExpr>, Z80ParserError> {
    let input_start = input.checkpoint();
    let input_offset = input.eof_offset();

    delimited(
        ("(", (my_space0, opt((line_ending, my_space0)))),
        separated(0.., located_expr, parse_comma_multiline),
        ((my_space0, opt((line_ending, my_space0))), ")")
    )
    .parse_next(input)
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_bool_value(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
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

    #[derive(Clone)]
    pub enum FactorModifier {
        High,
        Low,
        LogicalNot,
        BinaryNot
    };

    let (modifier, modifier_content) = opt(delimited(
        my_space0,
        alt((
            b'!'.value(FactorModifier::LogicalNot),
            (Caseless(b"NOT"), my_space0).value(FactorModifier::LogicalNot),
            b'~'.value(FactorModifier::BinaryNot),
            b'>'.value(FactorModifier::High),
            b'<'.value(FactorModifier::Low)
        )),
        my_space0
    ))
    .with_taken()
    .parse_next(input)?;

    let cloned = *input;
    let factor = preceded(
        my_space0,
        alt((
            positive_number,
            parse_bool_value,
            parse_label(false).map(|l| LocatedExpr::Label(l.into())),
            prefixed_label_expr,
            negative_number,
            parse_expr_bracketed_list.verify(|_| !is_orgams),
            // manage values
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
                .map(|((_m, dollar), content)| {
                    LocatedExpr::UnaryOperation(
                        UnaryOperation::Neg,
                        dollar,
                        cloned.update_slice(content).into()
                    )
                }),
            // manage labels
            parens
        )) /* ,
            * my_space0 */
    )
    .parse_next(input)?;

    let _ = my_space0(input)?;

    // if labels is followed by parenthesis, in fact it is a function call
    let factor = if factor.is_label()
        && peek::<_, _, Z80ParserError, _>(b'(')
            .parse_next(input)
            .is_ok()
    {
        let fname = factor.span();
        // specific content for specific functions
        if fname.as_str().eq_ignore_ascii_case("opcode") {
            // TODO move it in function handling and not operation
            let token =
                delimited(('(', my_space0), parse_token, (my_space0, ')')).parse_next(input)?;
            LocatedExpr::UnaryTokenOperation(
                UnaryTokenOperation::Opcode,
                Box::new(token),
                build_span(input_offset, &input_start, *input).into()
            )
        }
        else if fname.as_str().eq_ignore_ascii_case("duration") {
            let token =
                delimited(('(', my_space0), parse_token, (my_space0, ')')).parse_next(input)?;
            LocatedExpr::UnaryTokenOperation(
                UnaryTokenOperation::Duration,
                Box::new(token),
                build_span(input_offset, &input_start, *input).into()
            )
        }
        else {
            // general case
            let args = parse_function_arguments.parse_next(input)?;
            LocatedExpr::AnyFunction(
                factor.span().clone(),
                args,
                build_span(input_offset, &input_start, *input).into()
            )
        }
    }
    else {
        factor
    };

    let factor = match modifier {
        Some(FactorModifier::LogicalNot) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::Not,
                Box::new(factor),
                build_span(input_offset, &input_start, *input).into()
            )
        },
        Some(FactorModifier::BinaryNot) => {
            LocatedExpr::UnaryOperation(
                UnaryOperation::BinaryNot,
                Box::new(factor),
                build_span(input_offset, &input_start, *input).into()
            )
        },
        Some(FactorModifier::High) => {
            LocatedExpr::AnyFunction(
                (*input).update_slice(modifier_content).into(),
                vec![factor],
                build_span(input_offset, &input_start, *input).into()
            )
        },
        Some(FactorModifier::Low) => {
            LocatedExpr::AnyFunction(
                (*input).update_slice(modifier_content).into(),
                vec![factor],
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
pub fn fold_exprs(
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
            parse_oper(parse_factor, b"*", BinaryOperation::Mul),
            parse_oper(parse_factor, b"%", BinaryOperation::Mod),
            parse_oper(parse_factor, b"MOD", BinaryOperation::Mod),
            parse_oper(parse_factor, b"/", BinaryOperation::Div)
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
pub fn parse_oper<F>(
    inner: F,
    pattern: &'static [u8],
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

        // for orgams we cannot accept * as being a multiplication if it is followed by another * as it represents a repetition
        if input.state.options().is_orgams() && pattern == b"*" {
            not(pattern).parse_next(input)?;
        }

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
            parse_oper(shift, b"<=", BinaryOperation::LowerOrEqual),
            parse_oper(shift, b"<", BinaryOperation::StrictlyLower),
            parse_oper(shift, b">=", BinaryOperation::GreaterOrEqual),
            parse_oper(shift, b">", BinaryOperation::StrictlyGreater),
            parse_oper(shift, b"==", BinaryOperation::Equal),
            parse_oper(shift, b"=", BinaryOperation::Equal),
            parse_oper(shift, b"!=", BinaryOperation::Different)
        ))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(fold_exprs(initial, remainder, span))
}

pub fn expr(input: &mut InnerZ80Span) -> ModalResult<Expr, Z80ParserError> {
    located_expr
        .map(|e| e.to_expr().into_owned())
        .parse_next(input)
}
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
            parse_oper(expr2, b"&&", BinaryOperation::BooleanAnd),
            parse_oper(expr2, b"||", BinaryOperation::BooleanOr)
        ))
    )
    .parse_next(input)?;

    let span = build_span(input_offset, &input_start, *input);
    Ok(fold_exprs(initial, remainder, span))
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
        let _ = (Caseless(function_name), my_space0, ('('), my_space0).parse_next(input)?;

        let token = parse_token(input)?;

        let _ = (my_space0, ")").parse_next(input)?;

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
            parse_oper(comp, b"<<", BinaryOperation::LeftShift),
            parse_oper(comp, b">>", BinaryOperation::RightShift)
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
        parse_oper(term, b"+", BinaryOperation::Add),
        parse_oper(term, b"-", BinaryOperation::Sub),
        parse_oper(term, b"&", BinaryOperation::BinaryAnd), /* TODO check if it works and not compete with && */
        parse_oper(term, b"AND", BinaryOperation::BinaryAnd),
        parse_oper(term, b"|", BinaryOperation::BinaryOr), /* TODO check if it works and not compete with || */
        parse_oper(term, b"OR", BinaryOperation::BinaryOr),
        parse_oper(term, b"^", BinaryOperation::BinaryXor), /* TODO check if it works and not compete with ^^ */
        parse_oper(term, b"XOR", BinaryOperation::BinaryXor)
    ))).parse_next(input)?;

    Ok(fold_exprs(
        initial,
        remainder,
        build_span(start_eof_offset, &start, *input)
    ))
}

// Constants for label validation (orgams-specific)
const STAND_ALONE_DIRECTIVE_ORGAMS: &[&[u8]] = &[b"DB", b"DW", b"DS", b"ORG", b"EQU"];
const START_DIRECTIVE_ORGAMS: &[&[u8]] =
    &[b"REPEAT", b"REPT", b"MACRO", b"IF", b"IFDEF", b"IFNDEF"];
const END_DIRECTIVE_ORGAMS: &[&[u8]] = &[b"UNTIL", b"ENDM", b"ENDR", b"ENDIF"];

// Flag parsing functions
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

// Address parsing functions
/// Parse an address access `(expression)`
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_address(input: &mut InnerZ80Span) -> ModalResult<LocatedDataAccess, Z80ParserError> {
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

// Label parsing functions
pub fn parse_label(
    doubledots: bool
) -> impl Fn(&mut InnerZ80Span) -> ModalResult<InnerZ80Span, Z80ParserError> {
    #[cfg_attr(not(target_arch = "wasm32"), inline)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    move |input: &mut InnerZ80Span| {
        let start = input.checkpoint();

        let is_orgams = input.state.options().is_orgams();
        let obtained_label = if is_orgams {
            (
                opt(alt(("::", "@", "."))).value(()),
                alt((
                    one_of((
                        b'a'..=b'z',
                        b'A'..=b'Z',
                        b'_',
                        b'#',
                        b'\'' // orgams additions
                    ))
                    .value(()),
                    delimited('{', expr, '}').value(())
                )),
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((
                        take_while(
                            1..,
                            (
                                b'a'..=b'z',
                                b'A'..=b'Z',
                                b'0'..=b'9',
                                b'_',
                                b'#',
                                b'\'' // orgams additions
                            )
                        )
                        .value(()),
                        ".".value(()),
                        delimited('{', opt(expr), '}').value(())
                    ))
                )
            )
                .take()
                .parse_next(input)?
        }
        else {
            (
                opt(alt(("::", "@", "."))).value(()),
                alt((
                    one_of((b'a'..=b'z', b'A'..=b'Z', b'_')).value(()),
                    delimited('{', expr, '}').value(())
                )),
                repeat::<_, _, (), _, _>(
                    0..,
                    alt((
                        take_while(1.., (b'a'..=b'z', b'A'..=b'Z', b'0'..=b'9', b'_')).value(()),
                        ".".value(()),
                        delimited('{', opt(expr), '}').value(())
                    ))
                )
            )
                .take()
                .parse_next(input)?
        };

        let start_with_double_dots = obtained_label.len() > 2 && &obtained_label[..2] == b"::";
        let true_label = if start_with_double_dots {
            &obtained_label[2..]
        }
        else {
            obtained_label
        };

        // needed because of AT2
        let input = if doubledots {
            let _ = opt(Caseless(":")).parse_next(input)?;
            input
        }
        else {
            input
        };

        // Be sure that ::ld is not considered to be a label
        let label_len = true_label.len();
        if label_len >= MIN_MAX_LABEL_SIZE.0
            && label_len <= DOTTED_MIN_MAX_LABEL_SIZE.1
            && !ignore_ascii_case_allowed_label(
                true_label,
                input.state.options().dotted_directive,
                input.state.options().assembler_flavor
            )
        {
            input.reset(&start);
            Err(ErrMode::Backtrack(
                Z80ParserError::from_input(input).add_context(
                    input,
                    &start,
                    "You cannot use a directive or an instruction as a label"
                )
            ))
        }
        else {
            Ok((*input).update_slice(obtained_label))
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn ignore_ascii_case_allowed_label(
    name: &[u8],
    dotted_directive: bool,
    flavor: AssemblerFlavor
) -> bool {
    let len = name.len();

    // Get min/max and bucket function based on flavor and dotted directive
    let (min_max, bucket_fn): ((usize, usize), fn(usize) -> &'static [&'static str]) =
        match (flavor, dotted_directive) {
            (AssemblerFlavor::Basm, true) => {
                (DOTTED_MIN_MAX_LABEL_SIZE, dotted_impossible_by_length)
            },
            (AssemblerFlavor::Basm, false) => (MIN_MAX_LABEL_SIZE, impossible_by_length),
            (AssemblerFlavor::Orgams, _) => (ORGAMS_MIN_MAX_LABEL_SIZE, orgams_impossible_by_length)
        };

    let (min, max) = min_max;

    // Out-of-range lengths cannot match forbidden names
    if len < min || len > max {
        return true;
    }

    // Get bucket for this length
    let bucket = bucket_fn(len);

    // Check if name matches any forbidden name (case-insensitive)
    #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
    {
        use cpclib_common::rayon::prelude::*;
        return !bucket
            .par_iter()
            .any(|&content| content.as_bytes().eq_ignore_ascii_case(name));
    }

    #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
    {
        return !bucket
            .iter()
            .any(|&content| content.as_bytes().eq_ignore_ascii_case(name));
    }
}

/// Parser for file names in appropriate directives
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn parse_string(input: &mut InnerZ80Span) -> ModalResult<UnescapedString, Z80ParserError> {
    // Fast path keeps short strings on stack; heap is used only if escapes extend beyond the stack buffer.
    let opener = alt(('"', '\'')).parse_next(input)? as char;
    let closer = opener;
    let (normal, escapable) = match opener {
        '\'' => {
            (
                none_of(('\\', '\'')).take(),
                one_of(('\\', '\'', 'r', 'n', 't'))
            )
        },
        '"' => {
            (
                none_of(('\\', '"')).take(),
                one_of(('\\', '"', 'r', 'n', 't'))
            )
        },
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
    I: cpclib_common::winnow::stream::StreamIsPartial,
    I: Stream,
    I: AsBStr,
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
        let start = input.checkpoint();

        enum CollectedString {
            Owned(SmallVec<[u8; 64]>),
            Borrowed(&'static [u8], usize)
        }

        impl CollectedString {
            #[inline]
            fn new(start: &'static [u8]) -> Self {
                CollectedString::Borrowed(start, 0)
            }

            #[inline]
            fn extend_from_input_slice(&mut self, slice: &[u8]) {
                match self {
                    CollectedString::Owned(vec) => vec.extend_from_slice(slice),
                    CollectedString::Borrowed(_, len) => {
                        *len += slice.len();
                    }
                }
            }

            #[inline]
            fn extend_from_slice(&mut self, slice: &[u8]) {
                match self {
                    CollectedString::Owned(vec) => vec.extend_from_slice(slice),
                    CollectedString::Borrowed(..) => unreachable!()
                }
            }

            #[inline]
            fn extend_with_char(&mut self, c: u8) {
                match self {
                    CollectedString::Owned(vec) => vec.push(c),
                    CollectedString::Borrowed(i, len) => {
                        let mut vec = SmallVec::with_capacity(*len + 1);
                        vec.extend_from_slice(&i[..*len]);
                        vec.push(c);
                        *self = CollectedString::Owned(vec);
                    }
                }
            }

            #[inline]
            fn increment_borrowed_length(&mut self) {
                match self {
                    CollectedString::Owned(..) => unreachable!(),
                    CollectedString::Borrowed(_, len) => {
                        *len += 1;
                    }
                }
            }

            #[inline]
            fn as_slice(&self) -> &[u8] {
                match self {
                    CollectedString::Owned(vec) => vec.as_slice(),
                    CollectedString::Borrowed(i, len) => &i[..*len]
                }
            }

            #[inline]
            // by construction, the strings are valid utf8, so no need to check
            fn into_string(self) -> String {
                match self {
                    CollectedString::Owned(vec) => {
                        let vec = vec.into_vec();
                        unsafe { String::from_utf8_unchecked(vec) }
                    },
                    CollectedString::Borrowed(i, len) => unsafe {
                        String::from_utf8_unchecked(i[..len].to_vec())
                    }
                }
            }

            #[inline]
            fn is_borrowed(&self) -> bool {
                match self {
                    CollectedString::Owned(..) => false,
                    CollectedString::Borrowed(..) => true
                }
            }

            #[inline]
            fn transmute_to_owned_if_needed(self) -> Self {
                if let CollectedString::Borrowed(i, len) = self {
                    let mut vec: SmallVec<[u8; 64]> = SmallVec::with_capacity(len + 16);
                    vec.extend_from_slice(&i[..len]);
                    CollectedString::Owned(vec)
                }
                else {
                    self
                }
            }
        }

        let mut res: CollectedString =
            CollectedString::new(unsafe { std::mem::transmute(input.as_bstr()) });

        while input.eof_offset() > 0 {
            let current_len = input.eof_offset();

            if let Some(c) = opt(normal.by_ref().take()).parse_next(input)? {
                res.extend_from_input_slice(c.as_bytes()); // handle properly owned or borrowed version
                if input.eof_offset() == current_len {
                    return Ok(res.into_string());
                }
                continue;
            }

            if opt(control_char).parse_next(input)?.is_some() {
                if res.is_borrowed() {
                    res = res.transmute_to_owned_if_needed();
                }
                let c = escapable.parse_next(input)?;
                let c = c.as_char();
                match c {
                    'n' => res.extend_with_char(b'\n'),
                    'r' => res.extend_with_char(b'\r'),
                    't' => res.extend_with_char(b'\t'),
                    other => {
                        let mut buffer = [0; 4];
                        let s = other.encode_utf8(&mut buffer);
                        res.extend_from_slice(s.as_bytes());
                    }
                };
            }
            else {
                return Ok(res.into_string());
            }
        }

        input.reset(&start);
        input.finish();
        Ok(res.into_string())
    }
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

/// TODO
#[cfg_attr(not(target_arch = "wasm32"), inline)]
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn string_expr(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    parse_string.map(LocatedExpr::String).parse_next(input)
}
