	org 0x1000
	run $
beginning
	
	ld hl, data
loop
	ld a, (hl)
	or a : jp z, $
	call 0xbb5a
	inc hl
	jp loop

data
	db "HELLO WORLD FROM "
	ifdef BASM
	 db "basm"
	else
	 db "rasm"
	endif
	db 0

	save "TEST", beginning, $-beginning, AMSDOS