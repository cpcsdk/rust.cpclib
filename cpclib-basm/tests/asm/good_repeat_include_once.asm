	part = "once"
	repeat 3, count
		include once "include_{part}.asm"
	rend

		ld hl, unique_label