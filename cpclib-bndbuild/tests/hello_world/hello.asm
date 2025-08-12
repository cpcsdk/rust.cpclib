	org 0x4000
	run $
FIRST_ADDRESS

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
LAST_ADDRESS




	save "hello1.bin", FIRST_ADDRESS, LAST_ADDRESS-FIRST_ADDRESS, DSK, "hello1.dsk"

