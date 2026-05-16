
	; Test LZEF8 crunched section (bzpack EF8 format, forward)
	org 0x100

CS_START
	LZEF8
INNER_START
		defs 100, 0xaa
INNER_STOP
	LZCLOSE
CS_STOP

	assert INNER_STOP - INNER_START == 100
	assert CS_STOP - CS_START < 100
