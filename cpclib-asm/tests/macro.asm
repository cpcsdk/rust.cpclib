	macro addx,acc,ajout
		ld a,{acc}.low
		add {ajout}.low
		ld {acc}.low, a
		ld a,{acc}.high
		adc {ajout}.high
		ld {acc}.high, a
	mend

	addx bc, hl