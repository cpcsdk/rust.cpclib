
		org 0x400

VAR1 = 1
	assert VAR1 == 1
VAR1 = 3
	assert VAR1 == 3
	
	macro stuff
VAR2 = VAR1*3

		if VAR2 == 9
			db VAR1, VAR2
		else
			db VAR2, VAR1
		endif
	endm


	stuff (void)

VAR1 = 4

	stuff (void)

