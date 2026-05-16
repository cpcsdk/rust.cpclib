
	; Test INCLZM, INCEF8, INCBX0, INCBX2 directives (bzpack forward formats)
	org 0x4000

lzm_start
	INCLZM "good_all.bin"
lzm_end

ef8_start
	INCEF8 "good_all.bin"
ef8_end

bx0_start
	INCBX0 "good_all.bin"
bx0_end

bx2_start
	INCBX2 "good_all.bin"
bx2_end
