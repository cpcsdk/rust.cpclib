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