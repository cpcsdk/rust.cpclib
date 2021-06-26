use cpclib_asm::preamble::*;

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
		expr("18").unwrap().1,
		Expr::Value(18)
	);

	assert_eq!(
		expr("-18").unwrap().1,
		Expr::Value(-18)
	);
}

	
#[test]
fn db_negative_regression() {    

	let code = "	db 18";
	let listing  = parse_str(code).unwrap();
	assert_eq!(listing.len(), 1);
	assert_eq!(listing[0],
		Token::Defb(vec![Expr::Value(18)])
	);


	let code = "	db -18";
	let listing  = parse_str(code).unwrap();
	assert_eq!(listing.len(), 1);
	assert_eq!(listing[0],
		Token::Defb(vec![Expr::Value(-18)])
	);



}


#[test]
fn macro_args() {
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
	let listing  = dbg!(parse_str(code).unwrap());
	assert_eq!(listing.len(), 1);
	let token = listing.get(0).unwrap();
	assert_eq!(
		token.macro_name(),
		Some("CRC32XOR")
	);
	assert_eq!(
		token.macro_arguments().unwrap().len(),
		4
	);

}