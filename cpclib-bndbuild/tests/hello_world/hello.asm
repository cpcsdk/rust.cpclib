	org 0x4000
	run $

	ld hl, txt
loop
		ld a, (hl)
		or a
		jp z, $

		push hl
			call 0xbb5a
		pop hl
		inc hl
	jp loop

txt
	defb "Hello World!"
	defb 0