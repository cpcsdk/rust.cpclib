    ; ABYTE directive
    org $4000
    
    abyte 10, 100, 200
    
	assert memory(0x4000) == 100+10
	assert memory(0x4001) == 200+10
    ret
