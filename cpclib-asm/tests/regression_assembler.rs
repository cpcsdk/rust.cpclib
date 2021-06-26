use cpclib_asm::assemble;


#[test]
pub fn assemble_vsync_test() {
	let code = "
	org 0x4000
	ld b, 0xf5
loop
	in a, (c)
	rra
	jr nc, loop
end
	assert end == 0x4000 + (3+1+1+2)
	jr $
	";


	let binary = assemble(code).unwrap();

	assert_eq!(
		&binary,
		&[
			0x06, 0xf5,
			0xed, 0x78,
			0x1f,
			0x30, 0xfb,
			0x18, 0xfe
		]
	);
}

#[test]
pub fn macro_local_labels() {
	let code = "
	MACRO CRC32XOR x1,x2,x3,x4
	rr b
	jr nc,@nextBit
	  ld a,e
	  xor {x1}
	  ld e,a
	  ld a,d
	  xor {x2}
	  ld d,a
	  ld a,l
	  xor {x3}
	  ld l,a
	  ld a,h
	  xor {x4}
	  ld h,a
@nextBit
  MEND

	       CRC32XOR &2C,&61,&0E,&EE
		   CRC32XOR &19,&C4,&6D,&07
	";


	// just check that it assemble
	let binary = assemble(code).unwrap();
	assert!(binary.len() != 0);
}