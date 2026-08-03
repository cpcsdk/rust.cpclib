
	org 0x4000

; the comma-separated declaration style (no parentheses) also accepts a
; trailing ... to opt into variadic behavior
	macro sum3, a, b, ...
		db {a}, {b}, {2}, {#}
	endm

call1:
	sum3 1, 2, 3

	assert peek(call1) == 1
	assert peek(call1+1) == 2
	assert peek(call1+2) == 3
	assert peek(call1+3) == 3
