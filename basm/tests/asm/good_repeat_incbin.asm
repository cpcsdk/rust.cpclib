	
start
idx set 0
	repeat 26, count
		incbin 'AZERTY.TXT', idx, 1
idx set idx+1
	rend


	assert char(memory(start+0)) == 'A'
	assert char(memory(start+1)) == 'Z'
	assert char(memory(start+2)) == 'E'
	