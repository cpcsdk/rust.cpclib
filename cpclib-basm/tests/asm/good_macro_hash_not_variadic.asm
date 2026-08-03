
	org 0x4000

; without a trailing ..., {#} is NOT special - it's just literal text,
; exactly like any other unmatched {key} always was (backward compatibility)
	macro not_variadic(a)
call1:
		db {a}
		db string_len("{#}")
	endm

	not_variadic 9

; {#} substituted literally keeps the string content as the 3 characters
; `{`, `#`, `}` - if {#} had instead been (incorrectly) treated as the
; variadic arg-count form here, this would be `db string_len("1")` (a
; single-character string, 1 arg passed) instead
	assert peek(call1) == 9
	assert peek(call1+1) == 3
