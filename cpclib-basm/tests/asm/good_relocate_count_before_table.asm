; relocate_count is usable before the table is generated
	org 0x4000

	dw relocate_count
	dw relocate_size

	RELOCATE_START
start:
	ld hl, start
	call start
	ld (start), hl
	RELOCATE_END

	; not relocated: constants, relative jumps, 8 bits values
	ld hl, 0x1234
	jr start
	ld a, 5

	RELOCATE_TABLE

	assert relocate_count == 3
	assert relocate_size == 6
	assert peek(0x4000) == 3
	assert peek(0x4002) == 6
