
	org 0x4000
	
	macro magic(op, arg)
		{op} {arg}
	endm

	magic xor, (hl)
	magic add, b



magic2	macro(op, arg)
		{op} {arg}
	endm

	magic2 xor, (hl)
	magic2 add, b