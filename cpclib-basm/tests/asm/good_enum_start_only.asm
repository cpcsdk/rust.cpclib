; Explicit start, no step (defaults to 1)
; No prefix (empty before comma)
	enum ,5
A
B
C
	mend
	assert A == 5
	assert B == 6
	assert C == 7
