
	org 0x4000

; a named r# (raw string) param must still have its surrounding quotes
; stripped, exactly like a non-variadic macro's - but an *extra*, unnamed
; positional argument has no declared name to check for the r# convention,
; so it must never be stripped even if it happens to look like a string
	macro build_and_check(r#label, ...)
{label}_first
		db string_len("{label}")
		assert {1} == "raw_extra"
	endm

call1:
	build_and_check "HERE", "raw_extra"

	ifndef HERE_first
		fail "macro error: r# named param wasn't stripped"
	endif

; "HERE" is 4 characters - if the quotes had NOT been stripped from {label},
; string_len would instead be applied to the literal text `"HERE"` (with
; quotes baked into the source), which is not a valid string_len argument
; and would fail to assemble at all - so successfully reaching this byte
; check is itself part of the proof, on top of its actual value
	assert peek(call1) == 4
