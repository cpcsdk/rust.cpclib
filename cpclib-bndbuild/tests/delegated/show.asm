	BUILDSNA   ; Mandatory for rasm
	BANKSET 0  ; Mandatory for rasm

	org 0xc000
	incbin "martine.scr/MARTIN.SCR"

	org 0x4000
	run $

	di
	ld bc, 0x7f00
	ld hl, PAL

	repeat 4
		ld a, (hl)
		out (c), c : out (c), a
		inc c
		inc hl
	rend

	jp $

PAL 
	incbin "martine.scr/MARTIN.PAL", 3, 1
	incbin "martine.scr/MARTIN.PAL", 3+12, 1
	incbin "martine.scr/MARTIN.PAL", 3+12+12, 1
	incbin "martine.scr/MARTIN.PAL", 3+12+12+12, 1