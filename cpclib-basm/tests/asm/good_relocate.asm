; Port of the rasm autotest of RELOCATE_START/RELOCATE_END/RELOCATE_TABLE (WinAPE directives)
	org 0x1000

	defs 128
previous_os:
	defs 128

block_start:
	RELOCATE_START
	ld hl, some_data
some_label:
	ld a, (hl)
	or a
	jp nz, some_label
	jr more_code
more_code:
	jp os_function
	jp previous_os
some_data:
	db "hi there", 0
some_data_end:
length_of_some_data:
	dw some_data_end - some_data
pointer_to_some_data:
	dw some_data
pointer_inside_some_data:
	dw some_data + 5
	RELOCATE_END

os_function equ 0xF000
reltable_ofs:
	RELOCATE_TABLE

	; ld hl,some_data / jp nz,some_label / dw some_data / dw some_data+5
	assert reltable_ofs + 8 == $
	assert relocate_count == 4
	assert relocate_size == 8

	; positions of the 16 bits values of the relocatable block
	assert peek(reltable_ofs + 0) + 256 * peek(reltable_ofs + 1) == 0x1101 ; ld hl, some_data
	assert peek(reltable_ofs + 2) + 256 * peek(reltable_ofs + 3) == 0x1106 ; jp nz, some_label
	assert peek(reltable_ofs + 4) + 256 * peek(reltable_ofs + 5) == 0x111B ; dw some_data
	assert peek(reltable_ofs + 6) + 256 * peek(reltable_ofs + 7) == 0x111D ; dw some_data + 5

	; with a base address, with bytes
second_table:
	RELOCATE_TABLE byte block_start
	assert peek(second_table + 0) == 0x01
	assert peek(second_table + 1) == 0x06
	assert peek(second_table + 2) == 0x1B
	assert peek(second_table + 3) == 0x1D
	assert second_table + 4 == $
	assert relocate_size == 4
