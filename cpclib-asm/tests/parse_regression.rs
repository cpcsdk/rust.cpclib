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
		
		
