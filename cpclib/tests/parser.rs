#[macro_use]
extern crate matches;

#[cfg(test)]
mod tests {
    use std::ops::Deref;
    use std::u32;


    use cpclib_asm::preamble::*;
    use cpclib_common::lazy_static;
    use cpclib_common::nom::IResult;

    fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
        let mut ctx = Box::new(ParserContext::default());
        ctx.source = Some(code);
        ctx.context_name = Some("TEST".into());
        let span = Z80Span::new_extra(code, ctx.deref());
        (ctx, span)
    }


    fn get_opcode(code: &'static str) -> Token {
        let (_ctx, span) = ctx_and_span(code);
        match parse_org(span) {
            Err(e) => panic!("{:?}", e),
            Ok((_, opcode)) => opcode
        }
    }

    fn get_val<'src, 'ctx, T: core::fmt::Debug>(
        res: IResult<Z80Span, T, Z80ParserError>
    ) -> T {
        match res {
            Err(e) => panic!("{:?}", e),
            Ok((rest, val)) => {
                if rest.len() != 0 {
                    panic!("{:?} not assembled/ {:?}", rest, val);
                }
                val
            }
        }
    }

    fn is_error<T>(res: IResult<&str, T>) -> bool {
        match res {
            Err(_e) => true,
            Ok((..)) => false
        }
    }

    #[test]
    fn test_dec_number() {
        let (_ctx, span) = ctx_and_span("123");
        assert_eq!(get_val::<u32>(dec_number_inner(span)), 123);
    }

    #[test]
    fn test_bin_u16() {
        let (_ctx, span) = ctx_and_span("0b101011");

        assert_eq!(
            get_val::<u32>(bin_number_inner(span)),
            0b101011
        );
    }

    #[test]
    fn test_hex_number() {
        let (_ctx, span) = ctx_and_span("0x123");
        assert_eq!(
            get_val::<u32>(hex_number_inner(span)),
            0x123
        );

        let (_ctx, span) = ctx_and_span("0xffff");
        assert_eq!(
            get_val::<u32>(hex_number_inner(span)),
            0xFFFF
        );

        let (_ctx, span) = ctx_and_span("0x0000");
        assert_eq!(
            get_val::<u32>(hex_number_inner(span)),
            0x0000
        );

        let (_ctx, span) = ctx_and_span("0xc9fb");
        assert_eq!(
            get_val::<u32>(hex_number_inner(span)),
            0xC9FB
        );
    }

    #[test]
    #[should_panic]
    fn test_dec_number_neg() {
        let (_ctx, span) = ctx_and_span("-1");
        get_val::<u32>(dec_number_inner(span));
    }

    #[test]
    #[should_panic]
    fn test_hex_number_neg() {
        let (_ctx, span) = ctx_and_span("-0x0");
        get_val::<u32>(hex_number_inner(span));
    }

    #[test]
    fn test_expr() {
        let formula = "0xbd00 + 0x20 + 0b00001100";
        let (_ctx, span) = ctx_and_span(formula);

        let res = located_expr(span);
        assert!(res.is_ok());
        println!("{:?}", res);
        assert_eq!(
            res.ok().unwrap().1.eval().unwrap(),
            (0xBD00 + 0x20 + 0b00001100).into()
        );
    }

    #[test]
    fn test_org_value_decimal() {
        let code = "ORG 123";

        let opcode = get_opcode(code);
        assert!(opcode.org_expr().is_some());
        let arg1 = opcode.org_expr().unwrap();
        assert_eq!(arg1.eval().ok().unwrap(), 123.into());
    }

    #[test]
    fn test_org_value_hexadecimal() {
        let code = "ORG 0x123";

        let opcode = get_opcode(code);
        assert!(opcode.org_expr().is_some());
        let arg1 = opcode.org_expr().expect("expression expected");
        let value = arg1.eval();
        assert!(value.is_ok());
        // assert_matches!(arg1, &Expr::Value(0x123));
        assert_eq!(arg1.eval().ok().unwrap(), 0x123.into());
    }

    #[test]
    fn test_org_label() {
        let code = "ORG label\n";

        let opcode = get_opcode(code);
        assert!(opcode.org_expr().is_some());
        let arg1 = opcode.org_expr().unwrap();
        assert_eq!(arg1.is_context_independant(), false);
    }

    #[test]
    fn fn_test_label() {
        let (_ctx, span) = ctx_and_span("label");
        assert_eq!(
            parse_label(false)(span).ok().unwrap().1.as_str(),
            "label"
        );


        let (_ctx, span) = ctx_and_span("module.label");
        assert_eq!(
            parse_label(false)(span)
                .ok()
                .unwrap()
                .1.as_str(),
            "module.label"
        );

        let (_ctx, span) = ctx_and_span("label15");
        assert_eq!(
            parse_label(false)(span)
                .ok()
                .unwrap()
                .1.as_str(),
            "label15"
        );

        let (_ctx, span) = ctx_and_span(".label");
        assert_eq!(
            parse_label(false)(span).ok().unwrap().1.as_str(),
            ".label"
        );

        let code = "label";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 1);

        let code = "label";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = "label      \n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 1);

        let code = "demo_system_binary_start \n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_equ() {
        let code = "LABEL EQU VALUE";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 1);
    }

    #[test]
    #[should_panic]
    fn test_label_opcode() {
        let (_ctx, span) = ctx_and_span("ORG 0x1000");

        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], LocatedToken::Label(_));
    }

    #[test]
    fn fn_test_line() {
        let (_ctx, span) = ctx_and_span(" ");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 0);

        let (_ctx, span) = ctx_and_span(" ORG 0x1000");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0].to_token().into_owned(), Token::Org(_, None));

        let (_ctx, span) = ctx_and_span(" ORG 0x1000 ");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0].to_token().into_owned(), Token::Org(_, None));

        let (_ctx, span) = ctx_and_span("\tORG 0x1000");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0].to_token().into_owned(), Token::Org(_, None));

        let (_ctx, span) = ctx_and_span("    ORG 0x1000");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0].to_token().into_owned(), Token::Org(_, None));

        let (_ctx, span) = ctx_and_span(" ORG 0x1000; test");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0].to_token().into_owned(), Token::Org(_, None));

        let (_ctx, span) = ctx_and_span(" ORG 0x1000 ; test");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0].to_token().into_owned(), Token::Org(_, None));

        let (_ctx, span) = ctx_and_span("label ORG 0x1000");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], LocatedToken::Label(..));
        assert_matches!(tokens[1].to_token().into_owned(), Token::Org(..));

        let (_ctx, span) = ctx_and_span("label ORG 0x1000 : ORG 0x000 : ORG 10");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 4);
        assert_matches!(tokens[0], LocatedToken::Label(..));
        assert_matches!(tokens[1].to_token().into_owned(), Token::Org(..));
        assert_matches!(tokens[2].to_token().into_owned(), Token::Org(..));

        let (_ctx, span) = ctx_and_span("label ORG 0x1000 : ORG 0x000 : ORG 10 ; fdfs");
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 5);
        assert_matches!(tokens[0], LocatedToken::Label(..));
        assert_matches!(tokens[1].to_token().into_owned(), Token::Org(..));
        assert_matches!(tokens[2].to_token().into_owned(), Token::Org(..));

        let (_ctx, span) = ctx_and_span("label ORG 0x1000 ; : ORG 0x000 : ORG 10 ; fdfs");
        let tokens = get_val(parse_z80_line(span)
        );
        assert_eq!(tokens.len(), 3);
        assert_matches!(tokens[0], LocatedToken::Label(..));
        assert_matches!(tokens[1].to_token().into_owned(), Token::Org(..));
    }

    #[test]
    fn test_address() {
        let code = "(125)";
        let (_ctx, span) = ctx_and_span(code);
        let token = get_val(parse_address(span));
        assert_matches!(token, DataAccess::Memory(_));
    }
    #[test]
    fn test_ld() {
        let code = " ld hl, 0xc9fb\n";
        let (_ctx, span) = ctx_and_span(code);

        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld hl, 0xc9fb";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld de, 0xc9fb";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld de, (0xc9fb)";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld (0xc9fb),de";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld d, 0xc9";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld d, a";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " ld (hl), d";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_jump() {
        let code = " jr $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " jp 0xc9fb\n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " jr 0xc9fb";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " jp nz, 0xc9fb";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " jr nz, .other_lines";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = " jp nz, .other_lines + (9+4+1+2)     ; 4";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0].to_token().into_owned(), Token::OpCode(Mnemonic::Jp, _, _, _));
    }
    // #[test]
    // #[should_panic]
    // pub fn ld_16_8() {
    // XXX Currently do not panic as de can be considered has being a label
    // XXX Do we allow that ? The parse knows de is not a register but a label
    // XXX Do we forbid labels with the same name as a register ? => Probably better
    // let code = " ld hl, a";
    // let _tokens = get_val(parse_z80_line(span));
    // }

    // // deactivated -- there is a segfault however
    // #[test]
    // pub fn ld_16_16() {
    // let code = " ld hl, de";
    // let tokens = get_val(parse_z80_line(span));
    // println!("{:?}", tokens);
    // assert_matches!(
    // tokens[0].deref(),
    // Token::OpCode(
    // Mnemonic::Ld,
    // Some(DataAccess::Register16(_)),
    // Some(DataAccess::Register16(_)),
    // None
    // )
    // );
    // }

    #[test]
    #[should_panic]
    pub fn test_unique_lines_panic() {
        let line1 = "label1:fd";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 1);
    }

    #[test]
    pub fn test_unique_lines() {
        let line1 = "label1";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 1);

        let line1 = "label1  ";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 1);

        let line1 = "label1 ; blabla ";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 2);

        let line1 = "label1 ; blabla \n";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = parse_z80_line_complete(span).unwrap();
        assert_eq!(tokens.1.len(), 2);

        let line1 = "label1";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "label1  ";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "label1 ; blabla ";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 2);

        let line1 = "   org 0x100";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "  di";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ld hl, de"; // XXX
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ld (0x38), hl";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ei";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   jp $";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = "  org 0x100 :  di : ld hl, 0xc9fb : ld (0x38), hl : ei : jp $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 6);

        let code = "  org 0x100:di:ld hl, 0xc9fb:ld (0x38), hl :ei:jp $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 6);
    }

    #[test]
    pub fn test_res() {
        let line1 = "   res 7, a";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn fn_test_empty() {
        let code = "\n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_empty_line(span));
        assert_eq!(tokens.len(), 0);

        let code = ";with comment\n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_empty_line(span));
        assert_eq!(tokens.len(), 1);

        let code = "\n\n";
        let (_ctx, span) = ctx_and_span(code);
        let (code, _) = parse_empty_line(span).unwrap();
        assert_eq!(code.as_bytes(), b"\n");
        let (code, _) = parse_empty_line(code).unwrap();
        assert_eq!(code.as_bytes(), b"");

        let code = "\n\n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 0);

        let code = "";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 0);

        let code = "";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_empty_line(span));
        assert_eq!(tokens.len(), 0);

        let code = "  ";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_empty_line(span));

        let code = "  ";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 0);

        let code = "  ";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 0);

        let code = "  \n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 0);

        let code = "  ; comment \n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let code = "  ; comment \n";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(parse_empty_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn registers() {
        for reg in ["A", "B", "C", "D", "E", "H", "L"].iter() {
            let line = reg;
            let (_ctx, span) = ctx_and_span(line);
            get_val(parse_register8(span));
        }

        for reg in ["IXL", "IXH", "IYL", "IYH"].iter() {
            let line = reg;
            let (_ctx, span) = ctx_and_span(line);
            get_val(parse_indexregister8(span));
        }
    }

    #[test]
    fn test_add() {
        let line1 = "   ADD A, C";
        let (_ctx, span) = ctx_and_span(line1);
        let _tokens = get_val(parse_z80_line(span));

        let line1 = "   ADD A, IXL";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ADD HL, DE";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ADD A, (HL)";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ADD C";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_logical_operators() {
        let operators = ["xor", "and", "or"];
        let registers = ["a", "b", "c", "d", "e", "h", "l"];

        for op in operators.iter() {
            for reg in registers.iter() {
                let code = format!(" {} {}", op, reg);
                let line1 = unsafe{ &*(code.as_ref() as *const str) as &'static str};
                let (_ctx, span) = ctx_and_span(line1);
                let tokens = get_val(parse_z80_line(span));
                assert_eq!(tokens.len(), 1);
            }
        }
    }

    #[test]
    fn test_ret() {
        let line1 = "   RET";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET C";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET NC";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET Z";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET NZ";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET P";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET M";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET PE";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET PO";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_out() {
        let line1 = " OUT (C), D";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_lddr() {
        let line1 = " LDDR";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_ldir() {
        let line1 = " ldir";
        let (_ctx, span) = ctx_and_span(line1);
        let tokens = get_val(parse_z80_line(span));
        assert_eq!(tokens.len(), 1);
    }
    #[test]
    fn test_empty_repeat() {
        let z80 = "  repeat 5
            endrepeat
            ";
        let (_ctx, span) = ctx_and_span(z80);
        let res = dbg!(parse_repeat(span));
        assert!(res.is_ok());
    }

    #[test]
    fn test_empty_rept() {
        let z80 = "  rept 5
            endrepeat
            ";
        let (_ctx, span) = ctx_and_span(z80);
        let res = parse_repeat(span);
        assert!(res.is_ok());
    }

    #[test]
    fn test_empty_rep() {
        let z80 = "  rep 5
            endrepeat
            ";
            let (_ctx, span) = ctx_and_span(z80);

        let res = parse_repeat(span);
        assert!(res.is_ok());
    }

    #[test]
    fn test_call_macro() {
        let z80 = "MACRONAME";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(dbg!(parse_macro_or_struct_call(false, false)(
            span
        ))
        .is_err(),
    "Must fail because (void) is missing");

        let z80 = "BREAKPOINT_WINAPE";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(dbg!(parse_macro_or_struct_call(false, false)(
            span
        ))
        .is_err(),
        "Must fail because (void) is missing");

        let z80 = "MACRONAME 1, 2, 3, TRUC";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(parse_macro_or_struct_call(false, false)(span).is_ok());
        let z80 = "MACRONAME (void)";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(parse_macro_or_struct_call(false, false)(span).is_ok());

        let z80 = "LABEL MACRONAME";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(dbg!(parse_z80_line(span)).is_ok());
        let z80 = "LABEL MACRONAME 1, 2, \"trois\"";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(dbg!(parse_z80_line(span)).is_err());
        let z80 = "MACRONAME 1 2 3 ";
        let (_ctx, span) = ctx_and_span(z80);
        assert!(dbg!(parse_z80_line(span)).is_err());
    }

    #[test]
    fn test_repeat() {
        let z80 = "   repeat 5
                db 0
                db 1
            endrepeat
            ";
        let (_ctx, span) = ctx_and_span(z80);
        let res = dbg!(parse_repeat(span));
        assert!(res.is_ok());
    }

    #[test]
    fn test_repeat2() {
        let z80 = "\trepeat screen_height
		; Content for a single line runs on 128 nops
		pop hl ; 0 1
		xor a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 2 3
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 4 5
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 6 7
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 8 9
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 10 11
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 12 13
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 14 15
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h
    endr";
    let (_ctx, span) = ctx_and_span(z80);
        let res = parse_repeat(span);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0.len(), 0);
    }

    #[test]
    fn test_repeat3() {
        let z80 = "\trepeat screen_height
		; Content for a single line runs on 128 nops
		pop hl ; 0 1
		xor a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 2 3
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 4 5
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 6 7
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 8 9
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 10 11
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 12 13
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

		pop hl ; 14 15
		inc a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h
    endr";
    let (_ctx, span) = ctx_and_span(z80);
        let res = parse_z80_span(span);
        println!("{:?}", res);
        assert!(res.is_ok());
    }

    #[test]
    fn test_repeat4() {
        let z80 = "
        defs 10

        repeat screen_height
		; Content for a single line runs on 128 nops
		pop hl ; 0 1
		xor a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

    endr";
    let (_ctx, span) = ctx_and_span(z80);
        let res = parse_z80_span(span);
        println!("{:?}", res);
        assert!(res.is_ok());
    }

    #[test]
    fn test_in() {
        for reg in ["A", "B", "C", "D", "E", "H", "L"].iter() {
            let content = format!(" OUT (C), {}", reg);
            let line1 = unsafe{ &*(content.as_ref() as *const str) as &'static str};
        let (_ctx, span) = ctx_and_span(line1);
            let tokens = get_val(parse_z80_line(span));
            assert_eq!(tokens.len(), 1);
        }
    }

    #[test]
    fn test_prog_sample() {
        let code = "		ex af, af'
			dec a
			jr nz, player_line_loop";
            let (_ctx, span) = ctx_and_span(code);
        
        let res = parse_z80_span(span);
        println!("{:?}", &res);
        assert!(res.is_ok());
    }

    #[test]
    fn test_stableticker() {
        let code = "
        stableticker start stuff
            inc a
        stableticker stop";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        println!("{:?}", &res);
        assert!(res.is_ok());
        assert_eq!(3, res.unwrap().len());
    }

    #[test]
    fn test_duration() {
        let code = "
            ld hl, duration(inc hl)
            ld a, duration(ld a, label)
            defs  duration(inc hl) + thing
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        println!("{:?}", &res);
        assert!(res.is_ok());
        assert_eq!(3, res.unwrap().len());
    }

    #[test]
    fn test_opcode() {
        let code = "
            ld a, opcode(inc a)
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        assert!(res.is_ok());

        let code = "
            ld hl, opcode(inc a)
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        assert!(res.is_ok());

        let code = "
            ld a, opcode(ldd)
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        assert!(res.is_ok()); // Failure is detected in the assembler pass not the parser pass

        let code = "
            ld hl, opcode(ldd)
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        assert!(res.is_ok());

        let code = "
INC_L equ opcode(inc l)
INC_H equ opcode(inc h)
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        println!("{:?}", res);
        assert!(res.is_ok());

        let code = "
INC_L equ opcode(inc l)
INC_H equ opcode(inc h)
        ld a, INC_L
        ld (hl), a
        ";
        let (_ctx, span) = ctx_and_span(code);
        let res = parse_z80_span(span);
        println!("{:?}", res);
        assert!(res.is_ok());
    }

    #[test]
    fn fn_test_asm_prog1() {
        let code = "  org 0x100\n  di\n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n  jp $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 6);

        let code = "\n\n  org 0x100\n  di\n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = dbg!(parse_z80_span(span)).unwrap();
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 6);

        let code =
            "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $\n\n ";
            let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \nlabel ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $\n\n ";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 7);
    }

    #[test]
    fn test_basic_inclusion() {
        let code = "        LOCOMOTIVE toto, titi
10 ' fkdslfslkf
20 call {toto}
30 call {titi}
        ENDLOCOMOTIVE";
        let (_ctx, span) = ctx_and_span(code);

        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_basic_inclusion2() {
        let code = "        LOCOMOTIVE toto_1, titi_2
10 ' fkdslfslkf
20 call {toto_1}
30 call {titi_2}
        ENDLOCOMOTIVE";
        let (_ctx, span) = ctx_and_span(code);

        let tokens = parse_z80_span(span).unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_if() {
        let code = "IF expression
        ld a, b
        ld a, b
        ld a, b
    ENDIF";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = get_val(parse_conditional(span));

        let code = "IF expression
        ld a, b : ld a, b : ld a, b
    ENDIF";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = get_val(parse_conditional(span));

        let code = "IF expression : ld a, b : ld a, b : ld a, b
    ENDIF";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = get_val(parse_conditional(span));

        // TODO modify the parser to handle this case
        // let code = "IF expression : ld a, b : ld a, b : ld a, b : ENDIF";
        // println!("{:?}", parse_conditional(code));
        // let tokens = get_val(parse_conditional(code));

        let code = "IF expression
        ld a, b
        ld a, b
        call label
        ld a, b
    ENDIF";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = get_val(parse_conditional(span));

        let code = "\tIF expression
        ld a, b
        ld a, b
        call label
        ld a, b
    ENDIF";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();

        let code = "\t	if ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();

        let (_ctx, span) = ctx_and_span("
        \n\tif FDC_Is_Musical_Loader\r\n\t\tei\r\n\telse\r\n\t\tdi\r\n\tendif\n
        ");

        parse_z80_span(span).unwrap();

        let code = "\t	ifdef ENABLE_CATART_DISPLAY
		call blabla
	endif
    ";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span);

        let code = "\t	ifdef ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();

        let code = "\t	ifndef ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();

        let code = "\t	ifnot ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
    let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();

        let code =
            "\n    ifndef DEMOSYSTEM_ADDRESS\nDEMOSYSTEM_ADDRESS equ 0xC000 + 0x3200\n    org DEMOSYSTEM_ADDRESS\n    endif\n\n"
        ;
        let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();

        let code =
            "STACK_SIZE equ 20 ; XXX Very small stack; hope 10 calls is enough\n    ifndef DEMOSYSTEM_ADDRESS\nDEMOSYSTEM_ADDRESS equ 0xC000 + 0x3200\n    org DEMOSYSTEM_ADDRESS\n    endif\n\nSTACK_END"
        ;
        let (_ctx, span) = ctx_and_span(code);

        let _tokens = parse_z80_span(span).unwrap();
    }

    #[test]
    fn test_real_case() {
        let code = "
.first_line
                    ; end code : 9 nops
    pop de              ; 4
    ld a, e             ; 1
    dec sp              ; 2
    jp  .other_lines + (9+4+1+2) ; 4
.other_lines ; 0
    defs 64 - 4  ; 60
    dec a               ; 1
    jr nz, .other_lines ; 3
    ";

    let (_ctx, span) = ctx_and_span(code);

        let tokens = parse_z80_span(span).unwrap();
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 18);

        // XXX this one fails wereas it is the same than the previous one ! Only comments are removed
        let code = ".first_line
    pop de
    ld a, e
    dec sp
    jp  .other_lines + (9+4+1+2)
.other_lines
    defs 64 - 4
    dec a
    jr nz, .other_lines";
    let (_ctx, span) = ctx_and_span(code);


        // assemble line per line
        for line in code.split("\n").filter(|l| l.len() > 0) {
            println!("=> {}", line);
            let (_ctx, span) = ctx_and_span(line);

            let tokens = get_val(parse_z80_line(span));
            assert_eq!(tokens.len(), 1);
        }

        // Assemble in a whole
        let tokens = parse_z80_str(code).unwrap();
        println!("{:?}", &tokens);
        assert_eq!(tokens.len(), 9);
    }

    // XXX Stolen tests https://github.com/Geal/nom/blob/master/tests/arithmetic_ast.rs to check if
    // nothing is broken

    #[test]
    fn factor_test() {
        let (_ctx, span) = ctx_and_span("  3  ");
        let (input, res) = factor(span).unwrap();
        assert!(input.is_empty());
        assert_eq!(res.to_expr(), Expr::Value(3));
    }

    fn comp_test() {
        let (_ctx, span) = ctx_and_span("1 ");
        let (input, res) = comp(span).unwrap();
        assert!(input.is_empty());
        assert_eq!(res.to_expr(), Expr::Value(1));
    }

    #[test]
    fn term_test() {
        let (_ctx, span) = ctx_and_span(" 3 *  5   ");

        let (input, res) = term(span).unwrap();
        assert!(input.is_empty());
        assert_eq!(
            res.to_expr(),
            Expr::BinaryOperation(BinaryOperation::Mul, Box::new(Expr::Value(3)), Box::new(Expr::Value(5)))
        );
    }

    #[test]
    #[ignore = "Fail ATM, but no hurry to fix"]
    fn expr_test() {
        let (_ctx, span) = ctx_and_span(" 1 + 2 *  3 ");
        let (input, res) = located_expr(span).unwrap();
        assert!(input.is_empty());
        assert_eq!(
            res.to_expr(),
            Expr::BinaryOperation(BinaryOperation::Add,
                Box::new(Expr::Value(1)),
                Box::new(Expr::BinaryOperation(BinaryOperation::Mul,
                    Box::new(Expr::Value(2)),
                    Box::new(Expr::Value(3))
                ))
            )
        );

        let (_ctx, span) = ctx_and_span(" 1 + 2 *  3 / 4 - 5 ");
        let (input, res) = located_expr(span)
            .map(|(i, x)| (i, format!("{}", x.to_expr())))
            .unwrap();
        assert!(input.is_empty());
        assert_eq!(res, String::from("((0x1 + ((0x2 * 0x3) / 0x4)) - 0x5)"));

        let (_ctx, span) = ctx_and_span(" 72 / 2 / 3 ");
        let (input, res) = located_expr(span)
            .map(|(i, x)| (i, format!("{}", x.to_expr())))
            .unwrap();
        assert!(input.is_empty());
        assert_eq!(res, String::from("((0x48 / 0x2) / 0x3)"));
    }

    #[test]
    #[ignore = "Fail ATM, but no hurry to fix"]
    fn parens_test() {
        let (_ctx, span) = ctx_and_span(" ( 1 + 2 ) *  3 ");

        let (input, res) = located_expr(span)
            .map(|(i, x)| (i, format!("{}", x.to_expr())))
            .unwrap();
        assert!(input.is_empty());
        assert_eq!(res, String::from("(((0x1 + 0x2)) * 0x3)"));
    }

    #[test]
    fn functions_test() {
        let (_ctx, span) = ctx_and_span("lo(5)");
        let (input, res) = located_expr(span)
            .map(|(i, x)| (i, format!("{}", x.to_expr())))
            .unwrap();
        assert!(input.is_empty());
        assert_eq!(res, String::from("LO(0x5)"));
    }

    #[test]
    #[ignore = "Fail ATM, but no hurry to fix"]
    fn boolean_test() {
        let (_ctx, span) = ctx_and_span(" 0 == 1 ");

        let (input, res) = located_expr(span)
            .map(|(i, x)| (i, format!("{}", x.to_expr())))
            .unwrap();
        assert!(input.is_empty());
        assert_eq!(res, String::from(" 0 == 1 "))
    }

    // #[test]
    // fn include_test() {
    // let code = "  include \"file.asm\"";
    // let tokens = parse_z80_str(code).unwrap();
    // assert_eq!(tokens.len(), 1);
    // }

    #[test]
    fn quoted_string() {
        let code = "\"file.asm\"";
        let (_ctx, span) = ctx_and_span(code);
        let tokens = get_val(string_between_quotes(span));
        assert_eq!(tokens.to_string(), "file.asm".to_owned());

        let msg = "TODO -- Set the real address (in c7 space)";
        let code = format!("\"{}\"", &msg);
        let code = unsafe{ &*(code.as_ref() as *const str) as &'static str};
        let (_ctx, span) = ctx_and_span(&code);
        let tokens = get_val(string_between_quotes(span));
        assert_eq!(&tokens.to_string(), msg);
    }

    #[test]
    fn print_test() {
        let code = " PRINT 1";
        let tokens = parse_z80_str(code).unwrap();
        assert_eq!(tokens.len(), 1);

        let code = " PRINT 1-1";
        let tokens = parse_z80_str(code).unwrap();
        assert_eq!(tokens.len(), 1);

        let code = " PRINT 1 - 1";
        let tokens = parse_z80_str(code).unwrap();
        assert_eq!(tokens.len(), 1);

        let code =
            " PRINT zoomscroller_inject_for_step0_line_1 - zoomscroller_inject_for_step0_line_0";
        let tokens = parse_z80_str(code).unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn assert_test() {
        let code = " ASSERT 1\n";
        let (_ctx,code) = ctx_and_span(code);
        let token = dbg!(parse_assert(code.clone()));
        assert!(token.is_ok());

        let code = " ASSERT 1\n";
        let (_ctx,code) = ctx_and_span(code);
        let tokens = get_val(dbg!(parse_z80_line_complete(code)));
        assert_eq!(tokens.len(), 1);

        let code = " ASSERT 1 == 2";
        eprintln!("RES: {:?}", parse_z80_str(code));
        let (_ctx,code) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line_complete(code));
        assert_eq!(tokens.len(), 1);

        let code = " ASSERT 1 < 0x1000";
        eprintln!("RES: {:?}", parse_z80_str(code));
        let (_ctx,code) = ctx_and_span(code);
        let tokens = get_val(parse_z80_line_complete(code));
        assert_eq!(tokens.len(), 1);

        let code = " ASSERT 1 < 0x1000, \"blabla\"";
        let (_ctx,code) = ctx_and_span(code);
        let tokens = get_val(dbg!(parse_z80_line_complete(code)));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_db() {
        
        let code = "db Gfx1_bin_head, Gfx1_bin_track, Gfx1_bin_sector";
        let (_ctx, span) = ctx_and_span(code);

        get_val(parse_db_or_dw_or_str(span));

        let code = "db Gfx1_bin_head, Gfx1_bin_track, Gfx1_bin_sector and %1111, Gfx1_bin_size"
        ;
        let (_ctx, span) = ctx_and_span(code);
        get_val(parse_db_or_dw_or_str(span));
    }

    #[test]
    fn rorg() {
        let code = "\tRORG 1\n\tREND";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_rorg(span));

        let code = "\tRORG 1\n\tdb 5\n\tREND";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_rorg(span));

        let code = "\tRORG 1\n\tdb 5\n\tREND";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_rorg(span));
    }

    #[test]
    fn ld() {
        let code = "LD HL, 0";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_ld(span));

        let code = "LD HL, label";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_ld(span));

        let code = "LD HL, label_with_underscore";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = get_val(parse_ld(span));

        let code = "\tld hl, label_with_underscore";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = parse_z80_span(span).unwrap();

        let code = "\n\tld hl, label_with_underscore";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = parse_z80_span(span).unwrap();

        let code = "ld i,a";
        let (_ctx, span) = ctx_and_span(code);
        let _tokens = parse_ld(span);
    }

    #[test]
    fn expr_previous_failures() {
        let code = "  LD L, (IX + 0x0)";
        let token = &parse_z80_str(code).unwrap()[0];
        assert_eq!(token.mnemonic().unwrap(), &Mnemonic::Ld);
        assert_eq!(
            token.mnemonic_arg1().unwrap(),
            &DataAccess::Register8(Register8::L)
        );
        assert_eq!(
            token.mnemonic_arg2().unwrap(),
            &DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, Expr::Value(0))
        );

        let code = "  LD E, (HL)";
        let token = &parse_z80_str(code).unwrap()[0];
        assert_eq!(token.mnemonic().unwrap(), &Mnemonic::Ld);
        assert_eq!(
            token.mnemonic_arg1().unwrap(),
            &DataAccess::Register8(Register8::E)
        );
        assert_eq!(
            token.mnemonic_arg2().unwrap(),
            &DataAccess::MemoryRegister16(Register16::Hl)
        );

        let code = "  LD A, (DD)";
        let token = &parse_z80_str(code).unwrap()[0];
        assert_eq!(token.mnemonic().unwrap(), &Mnemonic::Ld);
        assert_eq!(
            token.mnemonic_arg1().unwrap(),
            &DataAccess::Register8(Register8::A)
        );
        assert_eq!(
            token.mnemonic_arg2().unwrap(),
            &DataAccess::Memory(Expr::Label("DD".into()))
        );
    }

    #[test]
    fn real_world_source_failures() {
        let (_ctx, span) = ctx_and_span("demosystem_binary_start");
        get_val(parse_label(false)(span));

        let (_ctx, span) = ctx_and_span("ld hl, demosystem_binary_start");
        get_val(parse_ld(span));

        let (_ctx, span) = ctx_and_span("\tprint \"TODO -- Set the real address (in c7 space)\"\n\tld hl, demosystem_binary_start\n\tld de, 0xDEAD\n\tld bc, demosystem_binary_stop - demosystem_binary_start\n");
        parse_z80_span(span).unwrap();

        let (_ctx, span) = ctx_and_span("
        ;This code is not used here, but can be useful to test the ST3 register of the FDC.
;Ret=A=ST3.
FDC_GetST3
	ld a,%00000100
	call FDC_PutFDC
	ld a,(FDC_Head)
	sla a
	sla a
	ld b,a
	ld a,(FDC_Drive)
	or b
	call FDC_PutFDC
	jr FDC_GetFDC
        ");

        parse_z80_span(span).unwrap();


        let (_ctx, span) = ctx_and_span("
        if FDC_Is_Musical_Loader
    call FDC_GotoTrack_NoWait
    ld a,5
    ld i,a	
    ei
    ld a,1
    ld (FDC_RS_Is_Interruption_On + 1),a
    call FDC_WaitEnd
else
    call FDC_GotoTrack
endif
");
        parse_z80_span(span).unwrap();
    }

    #[test]
    fn r#macro() {
        let (_ctx, span) = ctx_and_span("macro MYMACRO
        endm");
        get_val(parse_macro(span));

        let (_ctx, span) = ctx_and_span("    macro DEMOSYSTEM_SELECT_MAIN_BANKS_EXCEPT_SCREEN
        ld bc, 0x7fc1
        out (c), c
        ld (go_back_main_memory_from_extra_memory+1), bc
    endm");
        get_val(parse_macro(span));

        let (_ctx, span) = ctx_and_span("macro MYMACRO
        endm");
        parse_z80_span(span).unwrap();

        let (_ctx, span) = ctx_and_span("    macro DEMOSYSTEM_SELECT_MAIN_BANKS_EXCEPT_SCREEN
        ld bc, 0x7fc1
        out (c), c
        ld (go_back_main_memory_from_extra_memory+1), bc
    endm");
        parse_z80_span(span).unwrap();
    }
}
