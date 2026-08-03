
	org 0x4000

; a variadic macro called with exactly its named-argument count (no extras
; at all) must still work, with {#} equal to the named count
	macro sum3(a, b, ...)
		db {a}, {b}, {#}
	endm

call1:
	sum3 1, 2

	assert peek(call1) == 1
	assert peek(call1+1) == 2
	assert peek(call1+2) == 2
