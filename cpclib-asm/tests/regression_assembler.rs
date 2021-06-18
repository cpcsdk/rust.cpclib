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