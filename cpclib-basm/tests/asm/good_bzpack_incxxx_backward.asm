
	; Test INCLZM_BACKWARD, INCEF8_BACKWARD, INCBX0_BACKWARD, INCBX2_BACKWARD
	; (bzpack backward formats — only backward variants have a Z80 decruncher)
	org 0x4000

lzm_backward_start
	INCLZM_BACKWARD "good_all.bin"
lzm_backward_end

ef8_backward_start
	INCEF8_BACKWARD "good_all.bin"
ef8_backward_end

bx0_backward_start
	INCBX0_BACKWARD "good_all.bin"
bx0_backward_end

bx2_backward_start
	INCBX2_BACKWARD "good_all.bin"
bx2_backward_end


