
	; Test LZBX0_BACKWARD crunched section (bzpack BX0 backward format)
	org 0x100

CS_START
	LZBX0_BACKWARD
INNER_START
		defs 100, 0xaa
INNER_STOP
	LZCLOSE
CS_STOP

	assert INNER_STOP - INNER_START == 100
	assert CS_STOP - CS_START < 100

	; Provide symbols required by the decruncher
	SrcAddr = CS_START + BASM_LATEST_CRUNCH_OUTPUT_DATA_SIZE - 1
	DstAddr = 0xc000 + BASM_LATEST_CRUNCH_INPUT_DATA_SIZE - 1

	; Include the embedded decruncher
	include "inner://uncrunch/bzpack_backward_bx0.asm"
