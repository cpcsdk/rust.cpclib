
	; Illustration of a standard crunch section
	org 0x100

CS_START
	LZSA1
INNER_START
		defs 100
INNER_STOP
	LZCLOSE
CS_STOP

	assert INNER_STOP - INNER_START == 100
	assert CS_STOP - CS_START < 100