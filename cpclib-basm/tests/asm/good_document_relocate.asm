	org 0x4000

	; the loader needs to know how many addresses have to be fixed
	dw relocate_count

	RELOCATE_START
start
	ld hl, message        ; to be relocated
	call display            ; to be relocated
	ld bc, 0x1234         ; a constant: not relocated
	jr start              ; relative jump: not relocated
message
	db "Hello", 0
display
	ret
	RELOCATE_END

	; one word per value to relocate, relative to start
	RELOCATE_TABLE start

	assert relocate_count == 2
