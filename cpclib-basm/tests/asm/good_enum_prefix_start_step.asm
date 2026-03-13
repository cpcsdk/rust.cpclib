; RASM test: enum zib,10,10 → zib_dizaine==10, zib_vingtaine==20
	nop
	enum zib,10,10
dizaine
vingtaine
	mend
	assert zib_dizaine == 10
	assert zib_vingtaine == 20
