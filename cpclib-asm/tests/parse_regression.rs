use cpclib_asm::preamble::*;
static CTX: ParserContext = ParserContext {
    context_name: None,
    current_filename: None,
    read_referenced_files: false,
    search_path: Vec::new(),
};
#[test]
fn test_regression1() {
    let mut listing = Listing::new();

    let code = "	; Get source and destination address. Note that high byte destination should not been usefull
    pop hl
    pop de";

    let res = listing.add_code(code);
    println!("{:?}", res);
    assert!(res.is_ok());

    let mut listing = Listing::new();

    let code = "	
	; Get source and destination address. Note that high byte destination should not been usefull
    pop hl
    pop de";

    let res = listing.add_code(code);
    println!("{:?}", res);
    assert!(res.is_ok());

    let mut listing = Listing::new();

    let code = "
    ; Get source and destination address. Note that high byte destination should not been usefull
    pop hl
    pop de
    ";

    let res = listing.add_code(code);
    println!("{:?}", res);
    assert!(res.is_ok());
}

#[test]
fn expr_negative_regression() {
    assert_eq!(
        expr(CTX.build_span("18".to_owned())).unwrap().1,
        Expr::Value(18)
    );

    assert_eq!(
        expr(CTX.build_span("-18".to_owned())).unwrap().1,
        Expr::Value(-18)
    );
}

#[test]
fn db_negative_regression() {
    let code = "	db 18";
    let listing = parse_z80_str(code).unwrap();
    assert_eq!(listing.len(), 1);
    assert_eq!(
        *listing[0].clone().token().unwrap(),
        Token::Defb(vec![Expr::Value(18)])
    );

    let code = "	db -18";
    let listing = parse_z80_str(code).unwrap();
    assert_eq!(listing.len(), 1);
    assert_eq!(
        *listing[0].clone().token().unwrap(),
        Token::Defb(vec![Expr::Value(-18)])
    );
}

#[test]
fn macro_args1() {
    let code = "
	MACRO CRC32XOR x1,x2,x3,x4
	rr b
	jr nc,@nextBit
	  ld a,e
	  xor x1
	  ld e,a
	  ld a,d
	  xor x2
	  ld d,a
	  ld a,l
	  xor x3
	  ld l,a
	  ld a,h
	  xor x4
	  ld h,a
@nextBit
  MEND
	";
    let listing = dbg!(parse_z80_str(code).unwrap());
    assert_eq!(listing.len(), 1);
    let token = listing.get(0).unwrap();
    assert_eq!(
        token.clone().token().unwrap().macro_name(),
        Some("CRC32XOR")
    );
    assert_eq!(
        token
            .clone()
            .token()
            .unwrap()
            .macro_arguments()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn macro_args_single() {
    let code = "1".to_owned();
    let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

    assert_eq!(arg, MacroParam::Single("1".to_string()))
}

#[test]
fn macro_args_list_1() {
    let code = "[1]".to_owned();
    let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

    assert_eq!(
        arg,
        MacroParam::List(vec![Box::new(MacroParam::Single("1".to_string()))])
    )
}

#[test]
fn macro_args_list_2() {
    let code = "[1, 3]".to_owned();
    let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

    assert_eq!(
        arg,
        MacroParam::List(vec![
            Box::new(MacroParam::Single("1".to_string())),
            Box::new(MacroParam::Single(" 3".to_string())),
        ])
    )
}

#[test]
fn macro_args_list_3() {
    let code = "[1, ,3]".to_owned();
    let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

    assert_eq!(
        arg,
        MacroParam::List(vec![
            Box::new(MacroParam::Single("1".to_string())),
            Box::new(MacroParam::Single(" ".to_string())),
            Box::new(MacroParam::Single("3".to_string())),
        ])
    )
}


#[test]
fn regression_akm1() {

    let input = "IFDEF PLY_CFG_UseEffect_ArpeggioTable      ;CONFIG SPECIFIC
    ld de,PLY_AKM_PtArpeggios + PLY_AKM_Offset1b
    ldi
    ldi
                            ELSE
                            inc hl
                            inc hl
                            ENDIF ;PLY_CFG_UseEffect_ArpeggioTable
                            ";
    let bin = dbg!(parse_conditional(input.into()));
    assert!(bin.is_ok());
    dbg!(bin.unwrap().1.as_token());
}

#[test]
fn regression_akm2() {

        let input = "IFDEF PLY_CFG_UseEffect_PitchTable         ;CONFIG SPECIFIC
    ld de,PLY_AKM_PtPitches + PLY_AKM_Offset1b
    ldi
    ldi
        ELSE
        inc hl
        inc hl
        ENDIF ;PLY_CFG_UseEffect_PitchTable
";
    let bin = dbg!(parse_conditional(input.into()));
    assert!(bin.is_ok());
    dbg!(bin.unwrap().1.as_token());
}

#[test]
fn regression_akm3() {

        let input = "IFDEF PLY_CFG_UseEffects                           ;CONFIG SPECIFIC
        nop
    ELSE
        nop
    ENDIF ;PLY_CFG_UseEffects
";
    let bin = dbg!(parse_conditional(input.into()));
    assert!(bin.is_ok());
    dbg!(bin.unwrap().1.as_token());
}


#[test]
fn regression_akm4() {

        let input = "IFDEF PLY_CFG_UseEffects                           ;CONFIG SPECIFIC
        nop
    ELSE
dknr3:  ld de,4
    add hl,de
    ENDIF ;PLY_CFG_UseEffects
";
    let bin = dbg!(parse_conditional(input.into()));
    assert!(bin.is_ok());
    dbg!(bin.unwrap().1.as_token());
}


#[test]
fn regression_akm5() {

        let input = "IFDEF PLY_CFG_UseEffects                           ;CONFIG SPECIFIC
        IFDEF PLY_CFG_UseEffect_ArpeggioTable      ;CONFIG SPECIFIC
    ld de,PLY_AKM_PtArpeggios + PLY_AKM_Offset1b
    ldi
    ldi
            ELSE
            inc hl
            inc hl
            ENDIF ;PLY_CFG_UseEffect_ArpeggioTable
            IFDEF PLY_CFG_UseEffect_PitchTable         ;CONFIG SPECIFIC
    ld de,PLY_AKM_PtPitches + PLY_AKM_Offset1b
    ldi
    ldi
        ELSE
        inc hl
        inc hl
        ENDIF ;PLY_CFG_UseEffect_PitchTable
    ELSE
dknr3:  ld de,4
    add hl,de
    ENDIF ;PLY_CFG_UseEffects

";
    let bin = dbg!(parse_conditional(input.into()));
    assert!(bin.is_ok());
    dbg!(bin.unwrap().1.as_token());
}

#[test]
fn regression_crunched_section_sokoban() {
    let code = "


ENTITY_EMPTY = 1

ENTITY_FLOOR = 0
ENTITY_DESTINATION = 2
DEST_BIT = 1
ENTITY_BLOC = 4 ; ALWAYS in addition of floor or destination
BLOC_BIT = 2

ENTITY_VOID = 3

ENTITY_WALL = 8
WALL_BIT = 3
ENTITY_PLAYER = 16



	macro MAP_CHECK_BLOC bloc
		assert {bloc} == ENTITY_EMPTY || {bloc} == ENTITY_BLOC || {bloc} == ENTITY_DESTINATION || {bloc} == ENTITY_FLOOR || {bloc} == ENTITY_WALL || {bloc} == ENTITY_PLAYER || {bloc} == BD || {bloc} == ENTITY_VOID
	mend
	
	;;
	; Safely produce the data for a line of the map
	macro MAP_LINE9 a, b, c, d, e, f, g, h, i
		MAP_CHECK_BLOC {a}
		MAP_CHECK_BLOC {b}
		MAP_CHECK_BLOC {c}
		MAP_CHECK_BLOC {d}
		MAP_CHECK_BLOC {e}
		MAP_CHECK_BLOC {f}
		MAP_CHECK_BLOC {g}
		MAP_CHECK_BLOC {h}
		MAP_CHECK_BLOC {i}

		db E_
		db E_

		db {a}
		db {b}
		db {c}
		db {d}
		db {e}
		db {f}
		db {g}
		db {h}
		db {i}


		db E_
	mend

	macro MAP_LINE12 a, b, c, d, e, f, g, h, i,j,k,l
		MAP_CHECK_BLOC {a}
		MAP_CHECK_BLOC {b}
		MAP_CHECK_BLOC {c}
		MAP_CHECK_BLOC {d}
		MAP_CHECK_BLOC {e}
		MAP_CHECK_BLOC {f}
		MAP_CHECK_BLOC {g}
		MAP_CHECK_BLOC {h}
		MAP_CHECK_BLOC {i}

		db {a}
		db {b}
		db {c}
		db {d}
		db {e}
		db {f}
		db {g}
		db {h}
		db {i}

		db {j}
		db {k}
		db {l}
	mend

W_ = ENTITY_WALL
F_ = ENTITY_FLOOR
B_ = ENTITY_BLOC
D_ = ENTITY_DESTINATION
E_ = ENTITY_EMPTY
P_ = ENTITY_PLAYER


V_ = ENTITY_VOID

BD = B_ + D_

	macro MAP_EMPTY_LINE
		MAP_LINE9 E_,E_,E_,E_,E_,E_,E_,E_,E_
	mend

    LZAPU
.player_y db 5
.player_x db 6
        MAP_LINE12 E_,E_,E_,E_,W_,W_,W_,W_,W_,E_,E_,E_
        MAP_LINE12 E_,E_,W_,W_,W_,F_,F_,F_,W_,E_,E_,E_
        MAP_LINE12 E_,E_,W_,F_,F_,BD,W_,F_,W_,W_,E_,E_
        MAP_LINE12 E_,E_,W_,F_,W_,F_,F_,BD,F_,W_,E_,E_
        MAP_LINE12 E_,E_,W_,F_,BD,F_,F_,W_,F_,W_,E_,E_
        MAP_LINE12 E_,E_,W_,W_,F_,W_,D_,F_,F_,W_,E_,E_
        MAP_LINE12 E_,E_,E_,W_,F_,F_,F_,B_,W_,W_,E_,E_
        MAP_LINE12 E_,E_,E_,W_,W_,W_,F_,F_,W_,E_,E_,E_
        MAP_LINE12 E_,E_,E_,E_,E_,W_,W_,W_,W_,E_,E_,E_
        LZCLOSE
";

let bin = dbg!(parse_z80_str(code));
assert!(bin.is_ok());
}