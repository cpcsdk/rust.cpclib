
	org 0x4000
BINARY_START
	run $
	jp $
BINARY_STOP

	save "test", BINARY_START, BINARY_STOP-BINARY_START, DSK, "saved.dsk"