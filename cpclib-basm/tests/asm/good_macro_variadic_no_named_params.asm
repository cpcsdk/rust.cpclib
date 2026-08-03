
	org 0x4000

; a macro can be entirely variadic - no named parameters at all
	macro all_extra(...)
		db {#}
		db {0}, {1}, {2}
	endm

call1:
	all_extra 10, 20, 30

	assert peek(call1) == 3
	assert peek(call1+1) == 10
	assert peek(call1+2) == 20
	assert peek(call1+3) == 30

; a fully-variadic macro can also be called with zero arguments at all,
; as long as the body never references an index beyond what was given
	macro count_only(...)
		db {#}
	endm

call2:
	count_only (void)

	assert peek(call2) == 0
