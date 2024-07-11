
	macro dummy, res
		ld l, {res}
		; a useless comment to add some chars
	endm

	org 0x0000

	dummy hl
	dummy a
	dummy de