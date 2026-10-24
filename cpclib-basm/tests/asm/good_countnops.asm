; countnops(label1, label2): duration, in nops, of the code between two labels
	org 0x4000

start_nops:
	nop
	nop
	nop
end_nops:

start_mixed:
	ld a, b
	push bc
	pop bc
	inc hl
end_mixed:

start_loop:
	repeat 4
		nop
	endr
end_loop:

	assert countnops(start_nops, end_nops) == 3
	assert countnops(start_nops, start_nops) == 0
	assert countnops(start_loop, end_loop) == 4
	; same as the sum of the parts
	assert countnops(start_nops, end_loop) == countnops(start_nops, end_nops) + countnops(start_mixed, end_mixed) + countnops(start_loop, end_loop)
	assert countnops(start_mixed, end_mixed) == 1 + 4 + 3 + 2
