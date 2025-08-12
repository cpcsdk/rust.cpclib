	org 0x4000

SIZE1_start
	include once "include_once.asm"
SIZE1_stop

SIZE2_start
	include once "include_once.asm"
SIZE2_stop

	assert (SIZE1_stop - SIZE1_start) != 0
	assert (SIZE2_stop - SIZE2_start) == 0
