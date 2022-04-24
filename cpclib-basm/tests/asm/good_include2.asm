	org 0x4000

SIZE1_start
	include once "include_once.asm"
SIZE1_stop

SIZE2_start
	include once "include_once.asm"
SIZE2_stop

	assert (SIZE2_stop - SIZE2_start) == 0
