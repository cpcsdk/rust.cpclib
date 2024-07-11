	org 0x4000
BINARY_START
	run $
	ld a, 'A' : call 0xbb5a
	jp $
BINARY_STOP

	save "test", BINARY_START, BINARY_STOP-BINARY_START, HFE, "saved.hfe"