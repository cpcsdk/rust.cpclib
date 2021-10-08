	assert $ == 0
	assert $$ == 0

	org 0x100
	assert $ == 0x100
	assert $$ == 0x100
	nop
	assert $ == 0x101
	assert $$ == 0x101


	org 0x200, 0x300
	assert $ == 0x200
	assert $$ == 0x300
	nop
	assert $ == 0x201
	assert $$ == 0x301
