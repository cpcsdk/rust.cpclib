	macro LDIXREG register,dep
		if {dep}<-128 || {dep}>127
			push BC
			ld BC,{dep}
			add IX,BC
			ld (IX+0),{register}
			pop BC
		else
			ld (IX+{dep}),{register}
		endif
	mend

	LDIXREG H,200
	LDIXREG L,32