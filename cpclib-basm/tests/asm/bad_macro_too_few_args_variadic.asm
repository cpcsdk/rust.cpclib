
	org 0x4000

	macro sum3(a, b, ...)
		db {a}, {b}, {#}
	endm

; even a variadic macro still requires its named parameters - only one
; argument is provided here, but a and b are both mandatory
	sum3 1
