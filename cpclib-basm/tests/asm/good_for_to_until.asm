; FOR with the TO / UNTIL keywords (syntax of the fantams assembler)
	org 0x4000

first:
	for i = 0 to 3
		db {i}
	endfor
second:
	for j = 0 until 3
		db {j}
	endfor
third:
	for k = 0 until 10 step 4
		db {k}
	endfor
fourth:
	for old, 0, 2
		db {old}
	endfor
fifth:

	assert second - first == 4
	assert third - second == 3
	assert fourth - third == 3
	assert fifth - fourth == 3
	assert peek(first + 3) == 3
	assert peek(third + 2) == 8
