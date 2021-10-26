	org 0x4000

SIZE1_start
	include  "include_once.asm"
SIZE1_stop

SIZE2_start
	include  "include_once.asm" ; crash due to label definition
SIZE2_stop
