 macro m1
	db 1, 2, 3
	m2 4
 endm

 macro m2, arg
	ld hl, 0
	ld bc, 90
	rst {arg}
 endm

  m1