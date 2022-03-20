start
	repeat 3, count
		incbin 'AZERTY{{count}}.TXT'
	rend


	assert char(memory(start+0)) == 'A'
	assert char(memory(start+1)) == 'Z'
	assert char(memory(start+2)) == 'E'
	assert char(memory(start+3)) == 'R'
	assert char(memory(start+4)) == 'T'
	assert char(memory(start+5)) == 'Y'
	assert char(memory(start+6)) == 'U'
	assert char(memory(start+7)) == 'I'
	assert char(memory(start+8)) == 'O'
	assert char(memory(start+9)) == 'P'

	assert char(memory(start+10)) == 'Q'
	assert char(memory(start+11)) == 'S'
	assert char(memory(start+12)) == 'D'
	assert char(memory(start+13)) == 'F'
	assert char(memory(start+14)) == 'G'
	assert char(memory(start+15)) == 'H'
	assert char(memory(start+16)) == 'J'
	assert char(memory(start+17)) == 'K'
	assert char(memory(start+18)) == 'L'
	assert char(memory(start+19)) == 'M'


	assert char(memory(start+20)) == 'W'
	assert char(memory(start+21)) == 'X'
	assert char(memory(start+22)) == 'C'
	assert char(memory(start+23)) == 'V'
	assert char(memory(start+24)) == 'B'
	assert char(memory(start+25)) == 'N'

	