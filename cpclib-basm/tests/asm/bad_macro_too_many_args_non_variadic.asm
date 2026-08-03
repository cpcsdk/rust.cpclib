
	org 0x4000

	macro pair(a, b)
		db {a}, {b}
	endm

; pair is NOT declared variadic (no trailing ...) so a third argument must
; still be a hard arity error
	pair 1, 2, 3
