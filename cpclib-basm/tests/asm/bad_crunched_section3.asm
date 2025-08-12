; limits and protected are deactivated inside crunch section as we consider they have to be applied to the crunched version
; however, it is possible to duplicate it in the crunch section (and its life corresponds to the crosssection one only)


	org 0x100

	limit 0x1e0 ; limit not taken into account in the crunched section

	; Here the output address totally differs so we need to test the limit in the code space
	LZAPU
		assert $ == 0x100
		limit 0x1e1 ; limit taken into account in the crunched section
		defs 0x100 ; write over the limit => must fail
		assert $ == 0x200
	LZCLOSE
