
	; Illustration of nested crunch section
	org 0x100

CS_START

	LZAPU
	LZ49
	repeat 3
		LZ49
@INNER_START
			defs 100
@INNER_STOP
			assert @INNER_STOP - @INNER_START == 100

		LZCLOSE
	endr
	LZCLOSE
	db 3
	LZCLOSE

CS_STOP

	assert CS_STOP - CS_START < 100*3