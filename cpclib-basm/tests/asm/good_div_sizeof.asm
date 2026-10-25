; `div` operator and `sizeof` function (syntax of the fantams assembler)
	org 0x4000

	struct Point
x	db 0
y	db 0
	endstruct

divisor = 3                 ; a name starting with div is not the operator

	assert 7 div 2 == 3
	assert -7 div 2 == -3
	assert 7 DIV 2 == 3
	assert 7 / 2 == 3.5
	assert 7 mod 4 == 3
	assert divisor == 3
	assert sizeof(Point) == 2
	assert sizeof(Point) == Point
