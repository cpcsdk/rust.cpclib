	org 0x4000

	ld b, test
	djnz $

	include good_db.asm

	jp $

	binclude good_db.asm

test equ 5