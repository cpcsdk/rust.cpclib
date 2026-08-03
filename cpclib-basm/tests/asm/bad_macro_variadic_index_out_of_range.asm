
	org 0x4000

	macro sum3(a, b, ...)
		db {a}, {b}, {2}
	endm

; sum3's body references the first extra argument ({2}), but this call
; only supplies the two mandatory named ones - {2} has nothing to point at
	sum3 1, 2
