	snainit "../cpclib-sna/src/cpc6128.sna" ; should be uneeded by properly specifying sna properties
	
	org 0x4000
	run $

	

	ld hl, text_content
loop
		ld a, (hl)
		or a
		jp z, finished

		call TXT_OUTPUT
		inc hl
		jp loop

finished
	jp $

text_content
	db "Hello, world!", 0
	include "inner://firmware/txtvdu.asm"
