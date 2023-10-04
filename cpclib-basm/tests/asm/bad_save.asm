
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
.start
	defb "Hello World!"
	defb 0
.stop
LAST_ADDRESS


	save "hello.bin", FIRST_ADDRESS, LAST_ADDRESS-FIRST_ADDRESS, DSK, "hello.hfe" ; dsk != hfe


