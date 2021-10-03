; To be assembled, 3 passes are needed

	org 0x100

	ld hl, crunched_data
	ld de, 0x200
	assert crunched_data_size == 5 ; the value obtained here at the last pass; has to be correct and is only obtained with 3 pass
	; pass 1 => 0 because unknown
	; pass 2 => 0 because it is really 0 as it was not possible to do the defs
	; pass 3 => 5 because it has been properly computed in pass 2

	ld bc, crunched_data_size ; known at the end of pass 2 (so 3 passes are needed)
	ldir ; // should be the a call to uncrunch

crunched_data
	lzapu
		defs amount, value ; unknown at pass1
	lzclose
crunched_data_end

crunched_data_size equ crunched_data_end-crunched_data

amount equ 10
value equ 2

	assert crunched_data_size == 5 ; the value obtained at the very end