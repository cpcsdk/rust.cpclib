; RASM test: enum with no prefix, assert values
; lezero==0, leun==1
	nop
	enum
lezero
leun
	mend
	assert lezero == 0
	assert leun == 1
