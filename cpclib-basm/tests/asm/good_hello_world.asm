	org 0x4000

	include "inner://firmware/txtvdu.asm"

	ld hl, text_content
loop
		ld a, (hl)
		or a
		jp z, finished

		call TXT_OUTPUT
		jp loop

finished
	jp $

text_content
	db "Hello, world!", 0