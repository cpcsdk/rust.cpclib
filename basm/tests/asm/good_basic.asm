
	LOCOMOTIVE start
10 REM Basic loader of binary exec
20 REM yeah !!
30 call {start}
	ENDLOCOMOTIVE

start
		ld hl, txt
.loop
		ld a, (hl)
		or a : jr z, .end
		call #bb5a
		inc hl
		jr .loop
.end
		jp $

txt
	db "Hello world", 0

	print "LOADER START IN ", {hex}start
	save "LOADER.BAS",,,BASIC