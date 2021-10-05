
	org 0x1000

	ifndef toto
	assert toto == $
toto
		assert pouet == 0x1002
		dw pouet
	endif
pouet


