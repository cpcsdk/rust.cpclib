
	org 0x4000
	
	macro magic, op, arg
		{op} {arg}
	endm

	magic xor, (hl)
	magic add, b