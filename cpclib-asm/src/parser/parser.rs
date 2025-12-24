#![allow(clippy::cast_lossless)]

use core::str;

// use crc::*;
use super::context::*;
use super::*;
use crate::preamble::*;

include!(concat!(
    env!("OUT_DIR"),
    "/basm_directives_name_generated.rs"
));

// const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

pub use super::registers::{
    parse_indexregister_with_index, parse_indexregister8, parse_indexregister16, parse_register_i,
    parse_register_ix, parse_register_iy, parse_register_r, parse_register8, parse_register16
};

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

    use cpclib_common::winnow::Parser;
    use cpclib_common::winnow::ascii::line_ending;
    use cpclib_common::winnow::combinator::{repeat, terminated};
    use cpclib_common::winnow::error::{ErrMode, ParseError};
    use cpclib_common::winnow::stream::AsBStr;

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

    #[test]
    fn test_parse_factor_robust() {
        /// TODO extract all the strings in an array and loop over
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
        assert!(parse_test(parse_factor, "AB").is_ok());
        // Label with braces (we do not yet handle that. But we'll have too later)
        // assert!(parse_test(parse_factor, "label{macro}").is_ok());
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
        (
            "BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION",
            true
        ),
        ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)", true),
        (
            "(BREAKPOINT_METHOD == BREAKPOINT_WITH_SNAPSHOT_MODIFICATION)",
            true
        )
    ];

    #[test]
    fn test_parse_expr2_robust() {
        for (input, should_succeed) in EXPR2_CASES.iter().chain(COMP_CASES.iter()) {
            let result = parse_test(expr2, input);
            if *should_succeed {
                assert!(result.is_ok(), "Should parse '{}', got {:?}", input, result);
            }
            else {
                assert!(
                    result.is_err(),
                    "Should fail to parse '{}', got {:?}",
                    input,
                    result
                );
            }
        }
    }

    static COMP_CASES: &[(&str, bool)] = &[
        ("1+2", true),
        ("A+B", true),
        ("AB+BC", false), // BC is a register
        ("AB+CD", true),  // BC is a register
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
        ("A+B C", false)
    ];

    #[test]
    fn test_parse_comp_robust() {
        for (input, should_succeed) in COMP_CASES {
            let result = parse_test(comp, input);
            if *should_succeed {
                assert!(result.is_ok(), "Should parse '{}', got {:?}", input, result);
            }
            else {
                assert!(
                    result.is_err(),
                    "Should fail to parse '{}', got {:?}",
                    input,
                    result
                );
            }
        }
    }

    static LOCATED_EXPR_CASES: &[(&str, bool)] = &[
        ("BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES", true),
        ("(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)", true),
        (
            "(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)||(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)",
            true
        ),
        (
            "(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)||(BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES)",
            true
        ),
        (
            "((BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES) || (BREAKPOINT_METHOD == BREAKPOINT_WITH_WINAPE_BYTES))",
            true
        )
    ];

    #[test]
    fn test_parse_located_expression_robust() {
        for (input, should_succeed) in COMP_CASES
            .iter()
            .chain(EXPR2_CASES.iter())
            .chain(LOCATED_EXPR_CASES.iter())
        {
            let result = parse_test(located_expr, input);
            if *should_succeed {
                if result.is_err() {
                    eprintln!("FAIL: Should parse '{}', got {:?}", input, result);
                }
                assert!(result.is_ok(), "Should parse '{}', got {:?}", input, result);
            }
            else {
                if result.is_ok() {
                    eprintln!("FAIL: Should fail to parse '{}', got {:?}", input, result);
                }
                assert!(
                    result.is_err(),
                    "Should fail to parse '{}', got {:?}",
                    input,
                    result
                );
            }
        }
    }
}
