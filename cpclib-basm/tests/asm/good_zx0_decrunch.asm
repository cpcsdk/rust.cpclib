
	org 0x100


	ld hl, CS_START
	ld de, 0xc000
	call dezx0
	jp $

CS_START
	LZX0
INNER_START
		defs 100
INNER_STOP
	LZCLOSE
CS_STOP

	assert INNER_STOP - INNER_START == 100
	assert CS_STOP - CS_START < 100

	include "inner://dzx0_fast.asm"

dezx0
	DecompressZX0 (void)