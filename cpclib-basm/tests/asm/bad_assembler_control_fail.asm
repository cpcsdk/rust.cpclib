
	org 0x4000


	ASMCONTROLENV SET_MAX_NB_OF_PASSES=1
		ld hl, force_next_pass ; must fail because force_next_pass is unknown at first pass
	ENDASMCONTROLENV

force_next_pass
