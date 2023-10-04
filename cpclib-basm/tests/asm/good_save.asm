
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


	save "good_save_whole_inner.bin" ; Save binary without header
	save "hello.bin", FIRST_ADDRESS, LAST_ADDRESS-FIRST_ADDRESS, AMSDOS ; Save a binary with  header
	save "hello.bin", FIRST_ADDRESS, LAST_ADDRESS-FIRST_ADDRESS, DSK, "hello.dsk" ; Save binary with  header INSIDE a dsk
	save "hello.bin", FIRST_ADDRESS, LAST_ADDRESS-FIRST_ADDRESS, HFE, "hello.hfe" ; Save binary with  header INSIDE a hfe file

	save "good_save_txt.bin", txt.start, (txt.stop - txt.start) ; save text without header


; cmd line to generate the binary with header
;    basm good_save.asm --binary -o run.bin 
; cmd line to put it in a dsk
;    dskmanager test.dsk format --format data42
;    dskmanager test.dsk add run.bin 

