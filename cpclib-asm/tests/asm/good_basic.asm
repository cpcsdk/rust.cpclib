
	LOCOMOTIVE start
10 REM Basic loader of binary exec"
20 REM yeah !!
30 call {start}
	ENDLOCOMOTIVE

start
		jp $


	print "LOADER START IN", {hex}start
	save "LOADER.BAS",,,BASIC