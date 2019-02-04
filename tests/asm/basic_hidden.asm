;;
; Basic program that contains assembly code

	LOCOMOTIVE code
HIDE_LINES 20
10 ' Hello world
20 call {code}
	ENDLOCOMOTIVE

code

	di
		ld hl, 0xc9fb
		ld (0x38), hl
	ei
	
	ld bc, 0x7f10 : out (c), c
	ld a, 0x40 : out (c), a
	jp $