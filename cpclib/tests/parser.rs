#[macro_use]
extern crate matches;

#[cfg(test)]
mod tests {
    use cpclib_asm::preamble::*;
    use cpclib_sna::parse::*; // todo : move its tests at the right place

    use nom::IResult;

    fn check_mnemonic(code: &str, mnemonic: Mnemonic) -> bool {
        match parse_org(code) {
            Err(e) => panic!("{:?}", e),
            Ok((_, opcode)) => {
                let mut res = false;
                if let Token::OpCode(expected, _, _) = opcode {
                    res = expected == mnemonic
                }
                res
            }
        }
    }

    fn get_opcode(code: &str) -> Token {
        match parse_org(code) {
            Err(e) => panic!("{:?}", e),
            Ok((_, opcode)) => opcode,
        }
    }

    fn get_val<T: core::fmt::Debug>(res: IResult<&str, T>) -> T {
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
            Ok((_, _)) => false,
        }
    }

    #[test]
    fn test_dec_number() {
        assert_eq!(get_val::<u16>(dec_number("123")), 123);
    }

    #[test]
    fn test_bin_u16() {
        assert_eq!(get_val::<u16>(bin_u16("0b101011")), 0b101011);
    }

    #[test]
    fn test_hex_number() {
        assert_eq!(get_val::<u16>(hex_number("0x123")), 0x123);
        assert_eq!(get_val::<u16>(hex_number("0xffff")), 0xffff);
        assert_eq!(get_val::<u16>(hex_number("0x0000")), 0x0000);
        assert_eq!(get_val::<u16>(hex_number("0xc9fb")), 0xc9fb);
    }

    #[test]
    #[should_panic]
    fn test_dec_number_neg() {
        get_val::<u16>(dec_number("-1"));
    }

    #[test]
    #[should_panic]
    fn test_hex_number_neg() {
        get_val::<u16>(hex_number("-0x0"));
    }

    #[test]
    fn test_expr() {
        let formula = "0xbd00 + 0x20 + 0b00001100";
        let res = expr(formula);
        assert!(res.is_ok());
        println!("{:?}", res);
        assert_eq!(
            res.ok().unwrap().1.eval().unwrap(),
            0xbd00 + 0x20 + 0b00001100
        );
    }

    #[test]
    #[should_panic]
    fn test_dec_number_overflow() {
        get_val::<u16>(dec_number("65536"));
    }

    #[test]
    #[should_panic]
    fn test_hex_number_overflow() {
        get_val::<u16>(hex_number("0x10000"));
    }

    #[test]
    fn test_org_value_decimal() {
        let code = "ORG 123";

        let opcode = get_opcode(code);
        assert!(opcode.org_expr().is_some());
        let arg1 = opcode.org_expr().unwrap();
        assert_eq!(arg1.eval().ok().unwrap(), 123);
    }

    #[test]
    fn test_org_value_hexadecimal() {
        let code = "ORG 0x123";

        let opcode = get_opcode(code);
        assert!(opcode.org_expr().is_some());
        let arg1 = opcode.org_expr().expect("expression expected");
        let value = arg1.eval();
        assert!(value.is_ok());
        //assert_matches!(arg1, &Expr::Value(0x123));
        assert_eq!(arg1.eval().ok().unwrap(), 0x123);
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
        assert_eq!(parse_label(false)("label").ok().unwrap().1, "label");
        assert_eq!(parse_label(false)("label").ok().unwrap().1, "label");
        assert_eq!(parse_label(false)("module.label").ok().unwrap().1, "module.label");
        assert_eq!(parse_label(false)("label15").ok().unwrap().1, "label15");
        assert_eq!(parse_label(false)(".label").ok().unwrap().1, ".label");

        let code = "label";
        let tokens = get_val(parse_z80_line_label_only(code));
        assert_eq!(tokens.len(), 1);

        let code = "label";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = "label\n";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = "label      \n";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = "label      \n";
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 1);

        let code = "demo_system_binary_start \n";
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_equ() {
        let code = "LABEL EQU VALUE";
        let tokens = get_val(parse_z80_line_label_only(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    #[should_panic]
    fn test_label_opcode() {
        let tokens = get_val(parse_z80_line("ORG 0x1000"));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Label(_));
    }

    #[test]
    fn fn_test_line() {
        let tokens = get_val(parse_z80_line(" "));
        assert_eq!(tokens.len(), 0);

        let tokens = get_val(parse_z80_line(" ORG 0x1000"));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_, None));

        let tokens = get_val(parse_z80_line(" ORG 0x1000 "));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_, None));

        let tokens = get_val(parse_z80_line("\tORG 0x1000"));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_, None));

        let tokens = get_val(parse_z80_line("    ORG 0x1000"));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_, None));

        let tokens = get_val(parse_z80_line(" ORG 0x1000; test"));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::Org(_, None));

        let tokens = get_val(parse_z80_line(" ORG 0x1000 ; test"));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::Org(_, None));

        let tokens = get_val(parse_z80_line("label ORG 0x1000"));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_, _));

        let tokens = get_val(parse_z80_line("label ORG 0x1000 : ORG 0x000 : ORG 10"));
        assert_eq!(tokens.len(), 4);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_, _));
        assert_matches!(tokens[2], Token::Org(_, _));

        let tokens = get_val(parse_z80_line(
            "label ORG 0x1000 : ORG 0x000 : ORG 10 ; fdfs",
        ));
        assert_eq!(tokens.len(), 5);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_, _));
        assert_matches!(tokens[2], Token::Org(_, _));

        let tokens = get_val(parse_z80_line(
            "label ORG 0x1000 ; : ORG 0x000 : ORG 10 ; fdfs",
        ));
        assert_eq!(tokens.len(), 3);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_, _));
    }

    #[test]
    fn test_address() {
        let code = "(125)";
        let token = get_val(parse_address(code));
        assert_matches!(token, DataAccess::Memory(_));
    }
    #[test]
    fn test_ld() {
        let code = " ld hl, 0xc9fb\n";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld hl, 0xc9fb";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld de, 0xc9fb";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld de, (0xc9fb)";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld (0xc9fb),de";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld d, 0xc9";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld d, a";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " ld (hl), d";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_jump() {
        let code = " jr $";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " jp 0xc9fb\n";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " jr 0xc9fb";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " jp nz, 0xc9fb";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " jr nz, .other_lines";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = " jp nz, .other_lines + (9+4+1+2)     ; 4";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::OpCode(Mnemonic::Jp, _, _));
    }

    #[test]
    #[should_panic]
    pub fn ld_16_8() {
        // XXX Currently do not panic as de can be considered has being a label
        // XXX Do we allow that ? The parse knows de is not a register but a label
        // XXX Do we forbid labels with the same name as a register ? => Probably better
        let code = " ld hl, a";
        let _tokens = get_val(parse_z80_line(code));
    }

    #[test]
    pub fn ld_16_16() {
        let code = " ld hl, de";
        let tokens = get_val(parse_z80_line(code));
        println!("{:?}", tokens);
        assert_matches!(
            tokens[0],
            Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register16(_)),
                Some(DataAccess::Register16(_)),
            )
        );
    }

    #[test]
    #[should_panic]
    pub fn test_unique_lines_panic() {
        let line1 = "label1:fd";
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    pub fn test_unique_lines() {
        let line1 = "label1";
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "label1  ";
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "label1 ; blabla ";
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 2);

        let line1 = "label1 ; blabla \n";
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 2);

        let line1 = "label1";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "label1  ";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "label1 ; blabla ";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 2);

        let line1 = "   org 0x100";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "  di";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ld hl, de"; // XXX
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ld (0x38), hl";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ei";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   jp $";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let code = "  org 0x100 :  di : ld hl, 0xc9fb : ld (0x38), hl : ei : jp $";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 6);

        let code = "  org 0x100:di:ld hl, 0xc9fb:ld (0x38), hl :ei:jp $";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 6);
    }

    #[test]
    pub fn test_res() {
        let line1 = "   res 7, a";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn fn_test_empty() {
        let code = "\n";
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 0);

        let code = ";with comment\n";
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 1);

        let code = "\n\n";
        let (code, _) = parse_empty_line(code).unwrap();
        assert_eq!(code, "\n");
        let (code, _) = parse_empty_line(code).unwrap();
        assert_eq!(code, "");

        let code = "\n\n";
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = "";
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = "";
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 0);

        let code = "  ";
        let _tokens = get_val(parse_empty_line(code));

        let code = "  ";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 0);

        let code = "  ";
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = "  \n";
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = "  ; comment \n";
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = "  ; comment \n";
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn registers() {
        for reg in ["A", "B", "C", "D", "E", "H", "L"].iter() {
            let line = reg;
            get_val(parse_register8(line));
        }

        for reg in ["IXL", "IXH", "IYL", "IYH"].iter() {
            let line = reg;
            get_val(parse_indexregister8(line));
        }
    }

    #[test]
    fn test_add() {
        let line1 = "   ADD A, C";
        let _tokens = get_val(parse_z80_line(line1));

        let line1 = "   ADD A, IXL";
        println!("{:?}", parse_z80_line(line1));
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ADD HL, DE";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ADD A, (HL)";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   ADD C";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_logical_operators() {
        let operators = ["xor", "and", "or"];
        let registers = ["a", "b", "c", "d", "e", "h", "l"];

        for op in operators.iter() {
            for reg in registers.iter() {
                let code = format!(" {} {}", op, reg);
                let line1 = &code;
                let tokens = get_val(parse_z80_line(line1));
                assert_eq!(tokens.len(), 1);
            }
        }
    }

    #[test]
    fn test_ret() {
        let line1 = "   RET";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET C";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET NC";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET Z";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET NZ";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET P";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET M";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET PE";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = "   RET PO";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_out() {
        let line1 = " OUT (C), D";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_lddr() {
        let line1 = " LDDR";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_ldir() {
        let line1 = " ldir";
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }
    #[test]
    fn test_empty_repeat() {
        let z80 = "  repeat 5
            endrepeat
            ";
        let res = dbg!(parse_repeat(z80));
        assert!(res.is_ok());
    }

    #[test]
    fn test_empty_rept() {
        let z80 = "  rept 5
            endrepeat
            ";
        let res = parse_repeat(z80);
        assert!(res.is_ok());
    }

    #[test]
    fn test_empty_rep() {
        let z80 = "  rep 5
            endrepeat
            ";
        let res = parse_repeat(z80);
        assert!(res.is_ok());
    }

    #[test]
    fn test_call_macro() {
        let z80 = "MACRONAME";
        assert!(dbg!(parse_macro_call(z80.into())).is_ok());
        
        let z80 = "BREAKPOINT_WINAPE";
        assert!(dbg!(parse_macro_call(z80.into())).is_ok());

        let z80 = "MACRONAME 1, 2, 3, TRUC";
        assert!(parse_macro_call(z80.into()).is_ok());
        let z80 = "MACRONAME (void)";
        assert!(parse_macro_call(z80.into()).is_ok());

        let z80 = "LABEL MACRONAME";
        assert!(dbg!(parse_z80_line(z80.into())).is_ok());
        let z80 = "LABEL MACRONAME 1, 2, \"trois\"";
        assert!(dbg!(parse_z80_line(z80.into())).is_ok());
        let z80 = "MACRONAME 1 2 3 ";
        assert!(dbg!(parse_z80_line(z80.into())).is_err());
    }

    #[test]
    fn test_repeat() {
        let z80 = "   repeat 5
                db 0
                db 1
            endrepeat
            ";
        let res = dbg!(parse_repeat(z80.into()));
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
        let res = parse_repeat(z80);
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
        let res = parse_z80_code(z80);
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0.len(), 0);
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
        let res = parse_z80_code(z80);
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0.len(), 0);
    }

    #[test]
    fn test_in() {
        for reg in ["A", "B", "C", "D", "E", "H", "L"].iter() {
            let content = format!(" OUT (C), {}", reg);
            let line1 = &content;
            let tokens = get_val(parse_z80_line(line1));
            assert_eq!(tokens.len(), 1);
        }
    }

    #[test]
    fn test_prog_sample() {
        let code = "		ex af, af'
			dec a
			jr nz, player_line_loop";
        let res = parse_z80_code(code.into());
        println!("{:?}", &res);
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());
    }

    #[test]
    fn test_stableticker() {
        let code = "
        stableticker start stuff
            inc a
        stableticker stop";
        let res = parse_z80_code(code.into());
        println!("{:?}", &res);
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());
    }

    #[test]
    fn test_duration() {
        let code = "
            ld hl, duration(inc hl)
            ld a, duration(ld a, label)
            defs  duration(inc hl) + thing
        ";
        let res = parse_z80_code(code.into());
        println!("{:?}", &res);
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());
    }

    #[test]
    fn test_opcode() {
        let code = "
            ld a, opcode(inc a)
        ";
        let res = parse_z80_code(code.into());
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());

        let code = "
            ld hl, opcode(inc a)
        ";
        let res = parse_z80_code(code.into());
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());

        let code = "
            ld a, opcode(ldd)
        ";
        let res = parse_z80_code(code.into());
        assert!(res.is_ok()); // Failure is detected in the assembler pass not the parser pass
        assert_eq!(0, res.unwrap().0.len());

        let code = "
            ld hl, opcode(ldd)
        ";
        let res = parse_z80_code(code.into());
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());

        let code = "
INC_L equ opcode(inc l)
INC_H equ opcode(inc h)
        ";
        let res = parse_z80_code(code.into());
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());

        let code = "
INC_L equ opcode(inc l)
INC_H equ opcode(inc h)
        ld a, INC_L
        ld (hl), a
        ";
        let res = parse_z80_code(code.into());
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(0, res.unwrap().0.len());
    }

    #[test]
    fn fn_test_asm_prog1() {
        let code = "  org 0x100\n  di\n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 6);

        let code = "\n\n  org 0x100\n  di\n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 6);

        let code =
            "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $\n\n ";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \nlabel ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $\n\n ";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 7);
    }

    #[test]
    fn test_basic_inclusion() {
        let code = "        LOCOMOTIVE toto, titi
10 ' fkdslfslkf
20 call {toto}
30 call {titi}
        ENDLOCOMOTIVE";

        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_basic_inclusion2() {
        let code = "        LOCOMOTIVE toto_1, titi_2
10 ' fkdslfslkf
20 call {toto_1}
30 call {titi_2}
        ENDLOCOMOTIVE";

        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_if() {
        let code = "IF expression
        ld a, b
        ld a, b
        ld a, b
    ENDIF";
        println!("{:?}", parse_conditional(code));
        let _tokens = get_val(parse_conditional(code));

        let code = "IF expression
        ld a, b : ld a, b : ld a, b
    ENDIF";
        println!("{:?}", parse_conditional(code));
        let _tokens = get_val(parse_conditional(code));

        let code = "IF expression : ld a, b : ld a, b : ld a, b
    ENDIF";
        println!("{:?}", parse_conditional(code));
        let _tokens = get_val(parse_conditional(code));

        // TODO modify the parser to handle this case
        /*
        let code = "IF expression : ld a, b : ld a, b : ld a, b : ENDIF";
        println!("{:?}", parse_conditional(code));
        let tokens = get_val(parse_conditional(code));
        */

        let code = "IF expression
        ld a, b
        ld a, b
        call label
        ld a, b
    ENDIF";

        let _tokens = get_val(parse_conditional(code));

        let code = "\tIF expression
        ld a, b
        ld a, b
        call label
        ld a, b
    ENDIF";
        let _tokens = get_val(parse_z80_code(code));

        let code = "\t	if ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
        let _tokens = get_val(parse_z80_code(code));

        get_val(parse_z80_code(
            "
        \n\tif FDC_Is_Musical_Loader\r\n\t\tei\r\n\telse\r\n\t\tdi\r\n\tendif\n
        "
            .into(),
        ));

        let code = "\t	ifdef ENABLE_CATART_DISPLAY
		call blabla
	endif
    ";
        let _tokens = get_val(parse_z80_code(code));

        println!("{:?}", parse_label(false)("crtc_display_catart_if_needed".into()));
        println!(
            "{:?}",
            parse_z80_code(" call crtc_display_catart_if_needed".into())
        );

        let code = "\t	ifdef ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
        let _tokens = get_val(parse_z80_code(code));

        let code = "\t	ifndef ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
        let _tokens = get_val(parse_z80_code(code));

        let code = "\t	ifnot ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
    ";
        let _tokens = get_val(parse_z80_code(code));

        let code =
            "\n    ifndef DEMOSYSTEM_ADDRESS\nDEMOSYSTEM_ADDRESS equ 0xC000 + 0x3200\n    org DEMOSYSTEM_ADDRESS\n    endif\n\n"
        ;
        let _tokens = get_val(parse_z80_code(code));

        let code =
            "STACK_SIZE equ 20 ; XXX Very small stack; hope 10 calls is enough\n    ifndef DEMOSYSTEM_ADDRESS\nDEMOSYSTEM_ADDRESS equ 0xC000 + 0x3200\n    org DEMOSYSTEM_ADDRESS\n    endif\n\nSTACK_END"
        ;
        let _tokens = get_val(parse_z80_code(code));
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

        let tokens = get_val(parse_z80_code(code));
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

        // assemble line per line
        for line in code.split("\n").filter(|l| l.len() > 0) {
            println!("=> {}", line);
            let tokens = get_val(parse_z80_line(line));
            assert_eq!(tokens.len(), 1);
        }

        // Assemble in a whole
        let tokens = get_val(parse_z80_str(code));
        println!("{:?}", &tokens);
        assert_eq!(tokens.len(), 9);
    }

    // XXX Stolen tests https://github.com/Geal/nom/blob/master/tests/arithmetic_ast.rs to check if
    // nothing is broken

    #[test]
    fn factor_test() {
        assert_eq!(
            factor("  3  "),
            Ok(("", Expr::Value(3)))
        );
    }

    fn comp_test() {
        assert_eq!(
            comp("1 "),
            Ok(("", Expr::Value(1)))
        );
    }

    #[test]
    fn term_test() {
        assert_eq!(
            term(" 3 *  5   "),
            Ok(("", Expr::Mul(Box::new(Expr::Value(3)), Box::new(Expr::Value(5)))))
        );
    }

    #[test]
    fn expr_test() {
        assert_eq!(
            expr(" 1 + 2 *  3 "),
            Ok(("", Expr::Add(
                Box::new(Expr::Value(1)),
                Box::new(Expr::Mul(
                    Box::new(Expr::Value(2)),
                    Box::new(Expr::Value(3))
                ))
            )))
        );
        assert_eq!(
            expr(" 1 + 2 *  3 / 4 - 5 ").map(|(i, x)| (i, format!("{}", x))),
            Ok(("", String::from("((0x1 + ((0x2 * 0x3) / 0x4)) - 0x5)")))
        );
        assert_eq!(
            expr(" 72 / 2 / 3 ").map(|(i, x)| (i, format!("{}", x))),
            Ok(("", String::from("((0x48 / 0x2) / 0x3)")))
        );
    }

    #[test]
    fn parens_test() {
        assert_eq!(
            expr(" ( 1 + 2 ) *  3 ").map(|(i, x)| (i, format!("{}", x))),
            Ok(("", String::from("(((0x1 + 0x2)) * 0x3)")))
        );
    }

    #[test]
    fn functions_test() {
        assert_eq!(
            expr("lo(5)").map(|(i, x)| (i, format!("{}", x))),
            Ok(("", String::from("LO(0x5)")))
        );
    }

    #[test]
    fn boolean_test() {
        assert_eq!(
            expr(" 0 == 1 ").map(|(i, x)| (i, format!("{}", x))),
            Ok(("", String::from("0x0 == 0x1")))
        )
    }

    #[test]
    fn include_test() {
        let code = "  include \"file.asm\"";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn quoted_string() {
        let code = "\"file.asm\"";
        let tokens = get_val(string_between_quotes(code.into()));
        assert_eq!(tokens.to_string(), "file.asm".to_owned());

        let msg = "TODO -- Set the real address (in c7 space)";
        let code = format!("\"{}\"", &msg);
        let tokens = get_val(string_between_quotes(code.as_str().into()));
        assert_eq!(&tokens.to_string(), msg);
    }

    #[test]
    fn print_test() {
        let code = " PRINT 1";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        let code = " PRINT 1-1";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        let code = " PRINT 1 - 1";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        let code =
            " PRINT zoomscroller_inject_for_step0_line_1 - zoomscroller_inject_for_step0_line_0";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        get_val(parse_print(
            "PRINT \"TODO -- Set the real address (in c7 space)\"".into(),
        ));

        get_val(parse_z80_code(
            "\tPRINT \"TODO -- Set the real address (in c7 space)\"\n".into(),
        ));
    }

    #[test]
    fn assert_test() {
        let code = " ASSERT 1\n";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        let code = " ASSERT 1 == 2";
        eprintln!("RES: {:?}", parse_z80_str(code));
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        let code = " ASSERT 1 < 0x1000";
        eprintln!("RES: {:?}", parse_z80_str(code));
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

        let code = " ASSERT 1 < 0x1000, \"blabla\"";
        eprintln!("RES: {:?}", parse_z80_str(code));
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_db() {
        let code = "db Gfx1_bin_head, Gfx1_bin_track, Gfx1_bin_sector".into();
        get_val(parse_db_or_dw(code));

        let code =
            "db Gfx1_bin_head, Gfx1_bin_track, Gfx1_bin_sector and %1111, Gfx1_bin_size".into();
        get_val(parse_db_or_dw(code));
    }

    #[test]
    fn rorg() {
        let code = "\tRORG 1\n\tREND";
        let _tokens = get_val(parse_rorg(code.into()));

        let code = "\tRORG 1\n\tdb 5\n\tREND";
        let _tokens = get_val(parse_rorg(code.into()));

        let code = "\tRORG 1\n\tdb 5\n\tREND";
        let _tokens = get_val(parse_rorg(code.into()));
    }

    #[test]
    fn ld() {
        let code = "LD HL, 0";
        let _tokens = get_val(parse_ld(code.into()));

        let code = "LD HL, label";
        let _tokens = get_val(parse_ld(code.into()));

        let code = "LD HL, label_with_underscore";
        let _tokens = get_val(parse_ld(code.into()));

        let code = "\tld hl, label_with_underscore";
        let _tokens = get_val(parse_z80_code(code.into()));

        let code = "\n\tld hl, label_with_underscore";
        let _tokens = get_val(parse_z80_code(code.into()));

        let code = "ld i,a";
        let _tokens = get_val(parse_ld(code.into()));
    }

    #[test]
    #[should_panic]
    fn assert_test_should_assemble_later() {
        let code = " ASSERT 1 < 0x10000"; // ATM such number is not parsed
        eprintln!("RES: {:?}", parse_z80_str(code));
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn expr_previous_failures() {
        let code = "  LD L, (IX + 0x0)";
        let token = &get_val(parse_z80_str(code))[0];
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
        let token = &get_val(parse_z80_str(code))[0];
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
        let token = &get_val(parse_z80_str(code))[0];
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
        get_val(parse_label(false)("demosystem_binary_start".into()));
        get_val(parse_ld("ld hl, demosystem_binary_start".into()));
        get_val(parse_z80_code("\tprint \"TODO -- Set the real address (in c7 space)\"\n\tld hl, demosystem_binary_start\n\tld de, 0xDEAD\n\tld bc, demosystem_binary_stop - demosystem_binary_start\n".into()));

        get_val(parse_z80_code(
            "
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
        "
            .into(),
        ));

        get_val(parse_z80_code(
            "
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
    "
            .into(),
        ));
    }

    #[test]
    fn r#macro() {
        get_val(parse_macro(
            "macro MYMACRO
            endm"
                .into(),
        ));

        get_val(parse_macro(
            "    macro DEMOSYSTEM_SELECT_MAIN_BANKS_EXCEPT_SCREEN
        ld bc, 0x7fc1
        out (c), c
        ld (go_back_main_memory_from_extra_memory+1), bc
    endm"
                .into(),
        ));

        get_val(parse_z80_code(
            "macro MYMACRO
            endm"
                .into(),
        ));

        get_val(parse_z80_code(
            "    macro DEMOSYSTEM_SELECT_MAIN_BANKS_EXCEPT_SCREEN
        ld bc, 0x7fc1
        out (c), c
        ld (go_back_main_memory_from_extra_memory+1), bc
    endm"
                .into(),
        ));
    }
}
