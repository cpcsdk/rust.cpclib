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

txt incbin "data1.o" : incbin 'data2.o' 
	incbin "orgams/DATA3.BIN" 	; In opposite to what is written in UCPM 
								; fanzine, we are now able to remove the header
LAST_ADDRESS

	save UCPM_EXEC, \ ; Amsdos fname provided in command line
		FIRST_ADDRESS, \ ; Load address
		LAST_ADDRESS-FIRST_ADDRESS, \ ; Length
		DSK, \ ; request to save in a dsk
		UCPM_DSK ; dsk fname provided in command line
