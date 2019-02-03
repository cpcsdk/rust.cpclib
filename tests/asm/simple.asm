	org 0x4000

	di
		ld hl, 0xc9fb
		ld (0x38), hl
	ei
	
	ld bc, 0x7f10 : out (c), c
	ld a, 0x54 : out (c), a
	jp $