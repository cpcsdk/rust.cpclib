use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::combinator::{cut_err, opt, terminated};
use cpclib_common::winnow::stream::Stream;
use cpclib_common::winnow::token::take_until0;
use cpclib_common::winnow::{PResult, Parser};
use cpclib_tokens::Expr;

use super::{inner_code, InnerZ80Span, LocatedToken, LocatedTokenInner, Z80ParserError};
use crate::preamble::{
    located_expr, my_space0, my_space1, one_instruction_inner_code, parse_expr, parse_single_token,
    LocatedListing
};

pub static STAND_ALONE_DIRECTIVE_ORGAMS: &[&[u8]] = &[
    b"BANK", b"BRK", b"BYTE", b"DEFS", b"ELSE", //  b"END",
    b"ENT", b"IMPORT", b"ORG", b"PRINT",  b"SKIP", b"WORD"
];

pub static START_DIRECTIVE_ORGAMS: &[&[u8]] = &[b"IF", b"MACRO"];

pub static END_DIRECTIVE_ORGAMS: &[&[u8]] = &[b"END", b"ENDM", b"]"];

pub fn parse_orgams_fail(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
    let input_start = input.checkpoint();

    "!!".parse_next(input)?;

    let content = take_until0("\n").parse_next(input)?;
    let txt = String::from_utf8_lossy(content);
    let exp = Expr::String(SmolStr::new(txt));
    let fmtexp = cpclib_tokens::FormattedExpr::Raw(exp);
    let token = LocatedTokenInner::Fail(Some(vec![fmtexp]));

    let token = token.into_located_token_between(input_start, input.clone());

    Ok(token)
}

pub fn parse_orgams_repeat(input: &mut InnerZ80Span) -> PResult<LocatedToken, Z80ParserError> {
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
    let token = token.into_located_token_between(input_start, input.clone());

    Ok(token)
}

#[cfg(test)]
mod test {
    use std::ops::Deref;

    use cpclib_common::winnow::error::{ErrMode, ParseError};
    use cpclib_common::winnow::Parser;

    use crate::error::AssemblerError;
    use crate::preamble::{
        parse_line_component, InnerZ80Span, ParserContext, ParserContextBuilder, ParserOptions,
        Z80ParserError, Z80Span
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

    #[test]
    fn orgams_test_parse_macro_call() {
        assert!(dbg!(parse_test(parse_line_component, "empty (void)")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "empty(void)")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "empty()")).is_ok());
        assert!(dbg!(parse_test(parse_line_component, "empty ()")).is_ok());
    }
}
