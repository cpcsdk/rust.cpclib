	org 0x4000
	run $
FIRST_ADDRESS

	BREAKPOINT

	ld hl, txt
loop
	ld a, (hl) : or a : jp z, $
	call 0xbb5a
	inc hl
	jp loop

txt 	incbin "data1.o" : incbin 'data2.o' : defb 0
LAST_ADDRESS

	save "UCPM", FIRST_ADDRESS, \
		LAST_ADDRESS-FIRST_ADDRESS, \
		DSK, \
		"ucpm.dsk"
