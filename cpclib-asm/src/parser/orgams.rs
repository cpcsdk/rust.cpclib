use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::combinator::{alt, cut_err, delimited, not, opt, terminated};
use cpclib_common::winnow::stream::{AsBStr, Stream, UpdateSlice};
use cpclib_common::winnow::token::take_until;
use cpclib_common::winnow::{ModalResult, Parser};
use cpclib_tokens::{BinaryOperation, Expr};

use super::{
    InnerZ80Span, LocatedExpr, LocatedToken, LocatedTokenInner, Z80ParserError, inner_code,
    parse_factor
};
use crate::preamble::{MayHaveSpan, located_expr, my_space0, one_instruction_inner_code};

include!(concat!(
    env!("OUT_DIR"),
    "/orgams_directives_name_generated.rs"
));

pub fn parse_orgams_fail(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let input_start = input.checkpoint();

    "!!".parse_next(input)?;

    let content = take_until(0.., "\n").parse_next(input)?;
    let txt = String::from_utf8_lossy(content);
    let exp = Expr::String(SmolStr::new(txt));
    let fmtexp = cpclib_tokens::FormattedExpr::Raw(exp);
    let token = LocatedTokenInner::Fail(Some(vec![fmtexp]));

    let token = token.into_located_token_between(&input_start, *input);

    Ok(token)
}

pub fn parse_orgams_repeat(input: &mut InnerZ80Span) -> ModalResult<LocatedToken, Z80ParserError> {
    let input_start = input.checkpoint();

    let amount = terminated(located_expr, (my_space0, "**", my_space0)).parse_next(input)?;

    let bracket = opt('[').parse_next(input)?;
    let listing = if bracket.is_some() {
        my_space0.parse_next(input)?;
        let listing = cut_err(inner_code.context("ORGAMS REPEAT: unable to parse inner code"))
            .parse_next(input)?;
        ']'.parse_next(input)?;
        listing
    }
    else {
        one_instruction_inner_code.parse_next(input)?
    };

    let token = LocatedTokenInner::Repeat(amount, listing, None, None, None);
    let token = token.into_located_token_between(&input_start, *input);

    Ok(token)
}

#[inline]
pub fn parse_orgams_expression(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedExpr, Z80ParserError> {
    let mut factors = Vec::new();
    let mut operators = Vec::new();

    loop {
        factors.push(parse_orgams_ordered_expression.parse_next(input)?);

        if let Some(operator) =
            opt(delimited(my_space0, parse_orgams_operator, my_space0)).parse_next(input)?
        {
            operators.push(operator);
        }
        else {
            break;
        }
    }

    factors.reverse();
    operators.reverse();
    let mut expr = factors.pop().unwrap();
    while let Some(next) = factors.pop() {
        let operator = operators.pop().unwrap();
        let left = expr;
        let right = next;

        // span goes from left to right with operator between
        let left_bytes = left.span().as_bstr();
        let right_bytes = right.span().as_bstr();
        let size = (unsafe { right_bytes.as_ptr().byte_offset_from(left_bytes.as_ptr()) })
            .unsigned_abs()
            + right_bytes.len();
        let span = std::ptr::slice_from_raw_parts(left_bytes.as_ptr(), size);
        let span = (*input).update_slice(unsafe { &*span });

        expr = LocatedExpr::BinaryOperation(operator, Box::new(left), Box::new(right), span.into());
    }

    Ok(expr)
}

#[inline]
pub fn parse_orgams_operator(
    input: &mut InnerZ80Span
) -> ModalResult<BinaryOperation, Z80ParserError> {
    alt((
        '+'.value(BinaryOperation::Add),
        '-'.value(BinaryOperation::Sub),
        '/'.value(BinaryOperation::Div),
        ('*', not('*')).value(BinaryOperation::Mul),
        "AND".value(BinaryOperation::BinaryAnd),
        "OR".value(BinaryOperation::BinaryOr),
        "MOD".value(BinaryOperation::Mod)
    ))
    .parse_next(input)
}

#[inline]
pub fn parse_orgams_ordered_expression(
    input: &mut InnerZ80Span
) -> ModalResult<LocatedExpr, Z80ParserError> {
    let mut factors = Vec::new();
    let mut operators = Vec::new();

    loop {
        factors.push(parse_orgams_factor.parse_next(input)?);

        if let Some(operator) = opt(parse_orgams_operator).parse_next(input)? {
            operators.push(operator);
        }
        else {
            break;
        }
    }

    factors.reverse();
    operators.reverse();
    let mut expr = factors.pop().unwrap();
    while let Some(next) = factors.pop() {
        let operator = operators.pop().unwrap();
        let left = expr;
        let right = next;

        // span goes from left to right with operator between
        let left_bytes = left.span().as_bstr();
        let right_bytes = right.span().as_bstr();
        let size = (unsafe { right_bytes.as_ptr().byte_offset_from(left_bytes.as_ptr()) })
            .unsigned_abs()
            + right_bytes.len();
        let span = std::ptr::slice_from_raw_parts(left_bytes.as_ptr(), size);
        let span = (*input).update_slice(unsafe { &*span });

        expr = LocatedExpr::BinaryOperation(operator, Box::new(left), Box::new(right), span.into());
    }

    Ok(expr)
}

#[inline]
pub fn parse_orgams_factor(input: &mut InnerZ80Span) -> ModalResult<LocatedExpr, Z80ParserError> {
    let _before_bracket = input.checkpoint();
    let bracket = opt(('[', my_space0)).parse_next(input)?;
    let exp = if bracket.is_some() {
        let exp = parse_orgams_expression.parse_next(input)?;
        (my_space0, ']').parse_next(input)?;

        // TODO properly setup the string (we lack parenthesis)
        let span = exp.span().clone();
        LocatedExpr::Paren(Box::new(exp), span)
    }
    else {
        parse_factor(input)?
    };
    Ok(exp)
}

#[cfg(test)]
mod test {
    use std::ops::Deref;

    use cpclib_common::winnow::ModalParser;
    use cpclib_common::winnow::error::{ErrMode, ParseError};

    use crate::error::AssemblerError;
    use crate::preamble::{
        InnerZ80Span, ParserContext, ParserContextBuilder, ParserOptions, Z80ParserError, Z80Span,
        parse_line_component, parse_orgams_expression, parse_orgams_factor,
        parse_orgams_ordered_expression, parse_orgams_repeat
    };

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

    fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
        let mut options = ParserOptions::default();
        options.set_flavor(cpclib_tokens::AssemblerFlavor::Orgams);
        let ctx = Box::new(
            ParserContextBuilder::default()
                .set_context_name("ORGAMS_TEST")
                .set_options(options)
                .build(code)
        );
        let span = Z80Span::new_extra(code, ctx.deref());
        (ctx, span)
    }

    fn parse_test<O, P: ModalParser<InnerZ80Span, O, Z80ParserError>>(
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

    #[test]
    fn orgams_test_parse_macro_call() {
        assert!(dbg!(parse_test(parse_line_component, "empty (void)")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "empty(void)")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "empty()")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "empty ()")).is_ok());
    }

    // TODO check that the appropriate opcode is assembled
    #[test]
    fn orgams_test_parse_byte() {
        assert!(dbg!(parse_test(parse_line_component, "byte 1")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "db 2")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "defb 3")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "by 3")).is_ok());
    }

    #[test]
    fn orgams_test_expr() {
        assert!(dbg!(parse_test(parse_orgams_factor, "label")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_factor, "10")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_factor, "-$")).is_ok());

        assert!(dbg!(parse_test(parse_orgams_ordered_expression, "label")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_ordered_expression, "10")).is_ok());

        assert!(dbg!(parse_test(parse_orgams_ordered_expression, "label+10")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_ordered_expression, "label+10*5")).is_ok());

        assert!(dbg!(parse_test(parse_orgams_expression, "1+3 -5 *2")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_expression, "30 + #*2")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_expression, "&100 + #*10")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_expression, "96 MOD [30 + #*2]")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_expression, "[r1 + r13*2]*2")).is_ok());

        assert!(dbg!(parse_test(parse_orgams_expression, "SIN(10)")).is_ok());
        assert!(dbg!(parse_test(parse_orgams_expression, "ABS(SIN(10))")).is_ok());
    }

    #[test]
    fn orgams_test_repeat() {
        assert!(dbg!(parse_test(parse_orgams_repeat, "5 ** inc l")).is_ok());

        assert!(
            dbg!(parse_test(
                parse_orgams_repeat,
                "256 ** BYTE ABS(SIN(#)/256)"
            ))
            .is_ok()
        );
    }
}
