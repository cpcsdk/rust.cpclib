

	macro BUILD_LABEL r#label
{label}_first
	endm


	BUILD_LABEL "HERE"
	BUILD_LABEL "THERE"

	ifndef HERE_first
		fail "macro error"
	endif
	ifndef THERE_first
		fail "macro error"
	endif

	macro BUILD_CODE r#code
		{code}
	endm

START_CODE1
	BUILD_CODE "xor a"
	BUILD_CODE "ld hl, 0xc9fb : ld (0x38), hl"
END_CODE1

START_CODE2
	xor a
	ld hl, 0xc9fb : ld (0x38), hl
END_CODE2

	assert END_CODE2 - START_CODE2 == END_CODE1 - START_CODE1
	assert END_CODE2 - START_CODE2 == 7

	assert memory(START_CODE1) == memory(START_CODE2)
	assert memory(START_CODE1+1) == memory(START_CODE2+1)
	assert memory(START_CODE1+2) == memory(START_CODE2+2)
	assert memory(START_CODE1+3) == memory(START_CODE2+3)
	assert memory(START_CODE1+4) == memory(START_CODE2+4)
	assert memory(START_CODE1+5) == memory(START_CODE2+5)
	assert memory(START_CODE1+6) == memory(START_CODE2+6)

