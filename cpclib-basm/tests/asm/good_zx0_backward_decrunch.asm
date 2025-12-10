
	org 0x100
	run $
	breakpoint

	di

	ld hl, 0xc9fb : ld (0x38), hl
	ld sp, 0xc000

	; uncrunch the 100 bytes in 0xc000
	ld hl, CS_LAST_ADDRESS
	ld de, US_LAST_ADDRESS
	call dzx0_back
	jp $

	; create an area of 100 zero, but compress it
CS_START
	LZX0_BACKWARD
INNER_START
		defs 80, 0xff
		defs 20, 0x0f
INNER_STOP
	LZCLOSE
CS_STOP

	; Check that basm computes the right sizes
	assert CS_STOP-CS_START == BASM_LATEST_CRUNCH_OUTPUT_DATA_SIZE
	assert BASM_LATEST_CRUNCH_INPUT_DATA_SIZE == 100
	print 'Keep ', BASM_LATEST_CRUNCH_DELTA_SIZE, " bytes"

	; set up the needed symbols to uncruch
CS_LAST_ADDRESS = CS_START + BASM_LATEST_CRUNCH_OUTPUT_DATA_SIZE - 1 ; as in the doc. but does not work
US_LAST_ADDRESS = 0xc000 + BASM_LATEST_CRUNCH_INPUT_DATA_SIZE - 1 

	assert INNER_STOP - INNER_START == 100
	assert CS_STOP - CS_START < 100

	; include the embedded uncruncher
	include "inner://uncrunch/dzx0_turbo_back.asm"
dzx0_back
	dzx0_turbo_back (void)
