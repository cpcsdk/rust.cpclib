; Test case: 

	pop bc
	ld d,b
	ld e,c
	ld (var1),de
	ld (var2),bc
loop:
    jr loop

var1:
	dw 0
var2:
	dw 0