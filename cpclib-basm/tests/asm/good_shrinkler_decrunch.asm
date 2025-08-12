
	org 0x100

	di
		ld hl, 0xc9fb : ld (0x38), hl
	ei


	ld ix, CS_START
	ld de, 0xc000
	call shrinkler_decrunch
	jp $

CS_START
	LZSHRINKLER
INNER_START
		defs 100
INNER_STOP
	LZCLOSE
CS_STOP

	assert INNER_STOP - INNER_START == 100
	assert CS_STOP - CS_START < 100


	probs=0xc000
	include "inner://deshrink.asm"



	print "UNCRUNCHED SIZE=", INNER_STOP-INNER_START
	print "CRUNCHED SIZE=", CS_STOP-CS_START
