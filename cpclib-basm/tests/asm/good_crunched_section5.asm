	org 0x100
	
	lzapu
		ld bc, 0xbc00 + 4
		out (c), c
		ld bc, 0xbd00 + 0
		out (c), c
		ld bc, 0xbc00 + 9
		out (c), c
		ld bc, 0xbd00 + 0
		out (c), c
		ld bc, 0xbc00 + 7
		out (c), c
		ld bc, 0xbd00 + 255
		out (c), c
	lzclose


	lzapu
		ld bc, 0xbc00 + 4
		out (c), c
		inc b : ld c, 0
		out (c), c
		dec b : ld c, 9
		out (c), c
		inc b : ld c, 0
		out (c), c
		dec b : ld c, 7
		out (c), c
		inc b : ld c, 255
		out (c), c 
	lzclose


	lzapu
		ld hl, 0x0409
		ld de, 0x07FF
		ld bc, 0xbc00
		out (c), h
		inc b
		out (c), c
		dec b 
		out (c), l
		inc b 
		out (c), c
		dec b 
		out (c), d
		inc b
		out (c), e 
	lzclose
	db 0


	