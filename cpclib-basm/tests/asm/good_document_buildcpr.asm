	buildcpr

	bank 0
	print {hex}$
	print {hex}$$
	db 'A'


	bank 1
	org 0x8000
	print {hex}$
	print {hex}$$
	db 'B'

	;assert 0 == 1 => msut raise an assertion failure when uncommented
