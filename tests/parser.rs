


#[macro_use]
extern crate matches;

#[cfg(test)]
mod tests {
    use cpclib::assembler::parser::*;
    use cpclib::assembler::tokens::*;
    use nom::types::{CompleteStr, CompleteByteSlice};
    use nom::{ErrorKind, IResult, line_ending, space};

    fn check_mnemonic(code: &str, mnemonic: Mnemonic) -> bool {
        match parse_org(CompleteStr(code)) {
            Err(e) => panic!("{:?}", e),
            Ok( (_, opcode) )=> {
                let mut res = false;
                if let Token::OpCode(expected, _ , _) = opcode {
                    res = expected == mnemonic
                }
                res
            }
        }
    }


    fn get_opcode(code: &str) -> Token {
        match parse_org(CompleteStr(code)) {
            Err(e) => panic!("{:?}", e),
            Ok( (_, opcode) )=> {
                opcode
            }
        }
    }



    fn get_val<T>(res: IResult<CompleteStr<'_>, T>) -> T {
        match res {
            Err(e) => panic!("{:?}", e),
            Ok( (_, val) )=> {
                val
            }
        }
    }


    fn is_error<T>(res: IResult<CompleteStr<'_>, T>) -> bool {
        match res {
            Err(_e) => true,
            Ok( (_,_)) => false
        }
    }

    #[test]
    fn test_dec_u16() {
        assert_eq!(get_val::<u16>(dec_u16(CompleteStr("123"))), 123);
    }

    #[test]
    fn test_bin_u16() {
        assert_eq!(get_val::<u16>(bin_u16(CompleteStr("0b101011"))), 0b101011);
    }




    #[test]
    fn test_hex_u16() {
        assert_eq!(get_val::<u16>(hex_u16(CompleteStr("0x123"))), 0x123);
        assert_eq!(get_val::<u16>(hex_u16(CompleteStr("0xffff"))), 0xffff);
        assert_eq!(get_val::<u16>(hex_u16(CompleteStr("0x0000"))), 0x0000);
        assert_eq!(get_val::<u16>(hex_u16(CompleteStr("0xc9fb"))), 0xc9fb);
    }

    #[test]
    #[should_panic]
    fn test_dec_u16_neg() {

        get_val::<u16>(dec_u16(CompleteStr("-1")));
    }

    #[test]
    #[should_panic]
    fn test_hex_u16_neg() {

        get_val::<u16>(hex_u16(CompleteStr("-0x0")));
    }


    #[test]
    fn test_expr() {
        let formula = "0xbd00 + 0x20 + 0b00001100";
        let res = expr(CompleteStr(formula));
         assert_eq!(
             res.ok().unwrap().1.eval().unwrap(), 
             0xbd00 + 0x20 + 0b00001100);
    }

    #[test]
    #[should_panic]
    fn test_dec_u16_overflow() {
        get_val::<u16>(dec_u16(CompleteStr("65536")));
    }

    #[test]
    #[should_panic]
    fn test_hex_u16_overflow() {
        get_val::<u16>(hex_u16(CompleteStr("0x10000")));
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
        assert_eq!(parse_label(CompleteStr("label")).ok().unwrap().1, "label");
        assert_eq!(parse_label(CompleteStr("label")).ok().unwrap().1, "label");
        assert_eq!(parse_label(CompleteStr("module.label")).ok().unwrap().1, "module.label");
        assert_eq!(parse_label(CompleteStr("label15")).ok().unwrap().1, "label15");
        assert_eq!(parse_label(CompleteStr(".label")).ok().unwrap().1, ".label");



        let code = CompleteStr("label");
        let tokens = get_val(parse_z80_line_label_only(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr("label");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr("label\n");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr("label      \n");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr("label      \n");
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 1);


    }


    #[test]
    fn test_equ() {
        let code = CompleteStr("LABEL EQU VALUE");
        let tokens = get_val(parse_z80_line_label_only(code));
        assert_eq!(tokens.len(), 1);


    }

    #[test]
    #[should_panic]
    fn test_label_opcode() {
        let tokens = get_val(parse_z80_line(CompleteStr("ORG 0x1000")));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Label(_));
    }

    #[test]
    fn fn_test_line() {


        let tokens = get_val(parse_z80_line(CompleteStr(" ")));
        assert_eq!(tokens.len(), 0);


        let tokens = get_val(parse_z80_line(CompleteStr(" ORG 0x1000")));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_));

        let tokens = get_val(parse_z80_line(CompleteStr(" ORG 0x1000 ")));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_));

        let tokens = get_val(parse_z80_line(CompleteStr("\tORG 0x1000")));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_));

        let tokens = get_val(parse_z80_line(CompleteStr("    ORG 0x1000")));
        assert_eq!(tokens.len(), 1);
        assert_matches!(tokens[0], Token::Org(_));


        let tokens = get_val(parse_z80_line(CompleteStr(" ORG 0x1000; test")));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::Org(_));


        let tokens = get_val(parse_z80_line(CompleteStr(" ORG 0x1000 ; test")));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::Org(_));



        let tokens = get_val(parse_z80_line(CompleteStr("label ORG 0x1000")));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_));


        let tokens = get_val(parse_z80_line(CompleteStr("label ORG 0x1000 : ORG 0x000 : ORG 10")));
        assert_eq!(tokens.len(), 4);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_));
        assert_matches!(tokens[2], Token::Org(_));


        let tokens = get_val(parse_z80_line(CompleteStr("label ORG 0x1000 : ORG 0x000 : ORG 10 ; fdfs")));
        assert_eq!(tokens.len(), 5);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_));
        assert_matches!(tokens[2], Token::Org(_));


        let tokens = get_val(parse_z80_line(CompleteStr("label ORG 0x1000 ; : ORG 0x000 : ORG 10 ; fdfs")));
        assert_eq!(tokens.len(), 3);
        assert_matches!(tokens[0], Token::Label(_));
        assert_matches!(tokens[1], Token::Org(_));
    }


    #[test]
    fn test_address() {
        let code = CompleteStr("(125)");
        let token = get_val(parse_address(code));
        assert_matches!(token, DataAccess::Memory(_));
    }
    #[test]
    fn test_ld() {
        let code = CompleteStr(" ld hl, 0xc9fb\n");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" ld hl, 0xc9fb");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" ld de, 0xc9fb");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" ld de, (0xc9fb)");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);


        let code = CompleteStr(" ld (0xc9fb),de");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" ld d, 0xc9");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);


        let code = CompleteStr(" ld d, a");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);



        let code = CompleteStr(" ld (hl), d");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

    }

    #[test]
    fn test_jump() {
        let code = CompleteStr(" jr $");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);


        let code = CompleteStr(" jp 0xc9fb\n");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" jr 0xc9fb");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" jp nz, 0xc9fb");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" jr nz, .other_lines");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr(" jp nz, .other_lines + (9+4+1+2)     ; 4");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 2);
        assert_matches!(tokens[0], Token::OpCode(Mnemonic::Jp, _, _,));
    }


    #[test]
    #[should_panic]
    pub fn ld_16_8(){
        // XXX Currently do not panic as de can be considered has being a label
        // XXX Do we allow that ? The parse knows de is not a register but a label
        // XXX Do we forbid labels with the same name as a register ? => Probably better
        let code = CompleteStr(" ld hl, a");
        let _tokens = get_val(parse_z80_line(code));
    }

    #[test]
    pub fn ld_16_16(){
        let code = CompleteStr(" ld hl, de");
        let tokens = get_val(parse_z80_line(code));
        assert_matches!(
            tokens[0],
            Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register16(_)),
                Some(DataAccess::Register16(_))
            )
        );
    }

    #[test]
    #[should_panic]
    pub fn test_unique_lines_panic() {
        let line1 = CompleteStr("label1:fd");
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 1);
    }


    #[test]
    pub fn test_unique_lines() {
        let line1 = CompleteStr("label1");
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("label1  ");
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("label1 ; blabla ");
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 2);

        let line1 = CompleteStr("label1 ; blabla \n");
        let tokens = get_val(parse_z80_line_label_only(line1));
        assert_eq!(tokens.len(), 2);



        let line1 = CompleteStr("label1");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("label1  ");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("label1 ; blabla ");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 2);

        let line1 = CompleteStr("   org 0x100");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("  di");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   ld hl, de"); // XXX
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   ld (0x38), hl");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   ei");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   jp $");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr("  org 0x100 :  di : ld hl, 0xc9fb : ld (0x38), hl : ei : jp $");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 6);


        let code = CompleteStr("  org 0x100:di:ld hl, 0xc9fb:ld (0x38), hl :ei:jp $");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 6);
    }

    #[test]
    pub fn test_res() {
        let line1 = CompleteStr("   res 7, a");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }




    #[test]
    fn fn_test_empty() {
        let code = CompleteStr("\n\n");
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = CompleteStr("\n\n");
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 0);



        let code = CompleteStr("");
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = CompleteStr("");
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 0);

        let code = CompleteStr("  ");
        let _tokens = get_val(parse_empty_line(code));

        let code = CompleteStr("  ");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 0);

        let code = CompleteStr("  ");
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);

        let code = CompleteStr("  \n");
        let tokens = get_val(parse_z80_code(code));
        assert_eq!(tokens.len(), 0);


        let code = CompleteStr("  ; comment \n");
        let tokens = get_val(parse_z80_line(code));
        assert_eq!(tokens.len(), 1);

        let code = CompleteStr("  ; comment \n");
        let tokens = get_val(parse_empty_line(code));
        assert_eq!(tokens.len(), 1);
    }


    #[test]
    fn test_add() {
        let line1 = CompleteStr("   ADD A, C");
        let _tokens = get_val(parse_z80_line(line1));

        let line1 = CompleteStr("   ADD A, IXL");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   ADD HL, DE");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   ADD A, (HL)");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   ADD C");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }


    #[test]
    fn test_logical_operators() {

        let operators = [
            "xor", "and", "or"
        ];
        let registers = [
            "a", "b", "c", "d", "e", "h", "l"
        ];

        for op in operators.iter() {
            for reg in registers.iter() {
                let code = format!(" {} {}", op,reg);
                let line1 = CompleteStr(&code);
                let tokens = get_val(parse_z80_line(line1));
                assert_eq!(tokens.len(), 1);
            }
        }
    }

    #[test]
    fn test_ret() {
        let line1 = CompleteStr("   RET");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);


        let line1 = CompleteStr("   RET C");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);


        let line1 = CompleteStr("   RET NC");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   RET Z");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   RET NZ");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   RET P");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   RET M");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   RET PE");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);

        let line1 = CompleteStr("   RET PO");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_out() {
        let line1 = CompleteStr(" OUT (C), D");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }


    #[test]
    fn test_lddr() {
        let line1 = CompleteStr(" LDDR");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }

#[test]
    fn test_ldir() {
        let line1 = CompleteStr(" ldir");
        let tokens = get_val(parse_z80_line(line1));
        assert_eq!(tokens.len(), 1);
    }
    #[test]
    fn test_empty_repeat() {
        let z80 = "  repeat 5
            endrepeat
            ";
        let res = parse_repeat(CompleteStr(z80));
        assert!(res.is_ok());
    }

    #[test]
    fn test_empty_rept() {
        let z80 = "  rept 5
            endrepeat
            ";
        let res = parse_repeat(CompleteStr(z80));
        assert!(res.is_ok());
    }

    #[test]
    fn test_empty_rep() {
        let z80 = "  rep 5
            endrepeat
            ";
        let res = parse_repeat(CompleteStr(z80));
        assert!(res.is_ok());
    }




    #[test]
    fn test_repeat() {
        let z80 = "  repeat 5
                db 0
                db 1
            endrepeat
            ";
        let res = parse_repeat(CompleteStr(z80));
        assert!(res.is_ok());
    }


    #[test]
    fn test_repeat2() {
        let z80="\trepeat screen_height
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
        let res = parse_repeat(CompleteStr(z80));
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0.len(), 0);
    }


    #[test]
    fn test_repeat3() {
        let z80="\trepeat screen_height
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
        let res = parse_z80_code(CompleteStr(z80));
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0.len(), 0);
    }


    #[test]
    fn test_repeat4() {
        let z80="
        defs 10

        repeat screen_height
		; Content for a single line runs on 128 nops
		pop hl ; 0 1
		xor a : out (c), a : out (c), l
		inc a : out (c), a : out (c), h

    endr";
        let res = parse_z80_code(CompleteStr(z80));
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0.len(), 0);
    }

    #[test]
    fn test_in() {

        for reg in ["A", "B", "C", "D", "E", "H", "L"].iter() {
            let content  =format!(" OUT (C), {}", reg);
            let line1 = CompleteStr(&content);
            let tokens = get_val(parse_z80_line(line1));
            assert_eq!(tokens.len(), 1);
        }
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


        let code = "  \n  \n  org 0x100\n  di\n    \n ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $\n\n ";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 6);

        let code = "  \n  \n  org 0x100\n  di\n    \nlabel ld hl, 0xc9fb \n ld (0x38), hl\n ei \n jp $\n\n ";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 7);
    }


    #[test]
    fn test_real_case() {



        let code = CompleteStr("
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
    "
    );

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
        for line in code.split("\n").filter(|l| {l.len()>0}) {
            println!("=> {}", line);
            let line = CompleteStr(line);
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
            factor(CompleteStr("  3  ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("3")))
            );
    }

#[test]
    fn term_test() {
        assert_eq!(
            term(CompleteStr(" 3 *  5   ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("(3 * 5)")))
            );
    }

#[test]
    fn expr_test() {
        assert_eq!(
            expr(CompleteStr(" 1 + 2 *  3 ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("(1 + (2 * 3))")))
            );
        assert_eq!(
            expr(CompleteStr(" 1 + 2 *  3 / 4 - 5 ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("((1 + ((2 * 3) / 4)) - 5)")))
            );
        assert_eq!(
            expr(CompleteStr(" 72 / 2 / 3 ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("((72 / 2) / 3)")))
            );
    }

    #[test]
    fn parens_test() {
        assert_eq!(
            expr(CompleteStr(" ( 1 + 2 ) *  3 ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("([(1 + 2)] * 3)")))
            );


    }


    #[test]
    fn functions_test() {
        assert_eq!(
            expr(CompleteStr("lo(5)")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("LO(5)")))
            );
    }


    #[test]
    fn boolean_test() {
        assert_eq!(
            expr(CompleteStr(" 0 == 1 ")).map(|(i, x)| (i, format!("{:?}", x))),
            Ok((CompleteStr(""), String::from("0 == 1")))
        )
    }


    #[test]
    fn include_test() {
        let code = "  include \"file.asm\"";
        let tokens = get_val(parse_z80_str(code));
        assert_eq!(tokens.len(), 1);

    }

    #[test]
    fn assert_test() {
        let code = " ASSERT 1";
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
        assert_eq!(token.mnemonic_arg1().unwrap(), &DataAccess::Register8(Register8::L));
        assert_eq!(token.mnemonic_arg2().unwrap(), &DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, Oper::Add, Expr::Value(0)));

        let code = "  LD E, (HL)";
        let token = &get_val(parse_z80_str(code))[0];
        assert_eq!(token.mnemonic().unwrap(), &Mnemonic::Ld);
        assert_eq!(token.mnemonic_arg1().unwrap(), &DataAccess::Register8(Register8::E));
        assert_eq!(token.mnemonic_arg2().unwrap(), &DataAccess::MemoryRegister16(Register16::Hl));

        let code = "  LD A, (DD)";
        let token = &get_val(parse_z80_str(code))[0];
        assert_eq!(token.mnemonic().unwrap(), &Mnemonic::Ld);
        assert_eq!(token.mnemonic_arg1().unwrap(), &DataAccess::Register8(Register8::A));
        assert_eq!(token.mnemonic_arg2().unwrap(), &DataAccess::Memory(Expr::Label("DD".into())));




    }
}
