use cpclib_asm::preamble::*;
static CTX: ParserContext = ParserContext {
	context_name: None,
	current_filename: None,
	read_referenced_files: false,
	search_path: Vec::new()
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
	let listing  = parse_z80_str(code).unwrap();
	assert_eq!(listing.len(), 1);
	assert_eq!(*listing[0].clone().token().unwrap(),
		Token::Defb(vec![Expr::Value(18)])
	);


	let code = "	db -18";
	let listing  = parse_z80_str(code).unwrap();
	assert_eq!(listing.len(), 1);
	assert_eq!(*listing[0].clone().token().unwrap(),
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
	let listing  = dbg!(parse_z80_str(code).unwrap());
	assert_eq!(listing.len(), 1);
	let token = listing.get(0).unwrap();
	assert_eq!(
		token.clone().token().unwrap().macro_name(),
		Some("CRC32XOR")
	);
	assert_eq!(
		token.clone().token().unwrap().macro_arguments().unwrap().len(),
		4
	);
	

}

#[test]
fn macro_args_single() {
	let code = "1".to_owned();
	let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

	assert_eq!(
		arg,
		MacroParam::Single("1".to_string())
	)
}

#[test]
fn macro_args_list_1() {
	let code = "[1]".to_owned();
	let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

	assert_eq!(
		arg,
		MacroParam::List(
			vec![
				Box::new(MacroParam::Single("1".to_string()))
			]
		)
	)
}

#[test]
fn macro_args_list_2() {
	let code = "[1, 3]".to_owned();
	let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

	assert_eq!(
		arg,
		MacroParam::List(
			vec![
				Box::new(MacroParam::Single("1".to_string())),
				Box::new(MacroParam::Single(" 3".to_string())),
			]
		)
	)
}

#[test]
fn macro_args_list_3() {
	let code = "[1, ,3]".to_owned();
	let arg = dbg!(parse_macro_arg(CTX.build_span(code))).unwrap().1;

	assert_eq!(
		arg,
		MacroParam::List(
			vec![
				Box::new(MacroParam::Single("1".to_string())),
				Box::new(MacroParam::Single(" ".to_string())),
				Box::new(MacroParam::Single("3".to_string())),
			]
		)
	)
}