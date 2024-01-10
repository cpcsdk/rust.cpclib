
	org #1000

	ld hl, txt
loop
	ld a, (hl)
	or a
	jp $
	call #bb5a
	inc hl
	jr loop

txt byte "HELLO WORLD"
   byte 0