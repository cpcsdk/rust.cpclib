
	; Takes inspiration from BRASS assembler

	for count, 0, 10, 3
		db {count}
	endfor

	for x, 0, 3
		for y, 0, 3
			db {x}*4 + {y}
		fend
	endfor