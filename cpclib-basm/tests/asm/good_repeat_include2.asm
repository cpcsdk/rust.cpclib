
	
start	
	repeat 3, count
		include "included_{{count}}.asm"
	rend

	assert memory (start+0) == 1
	assert memory (start+1) == 2
	assert memory (start+2) == 3

var = 1
		include "included_{var}.asm"
