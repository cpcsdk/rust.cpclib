	ld a, [hl]
	ld [hl], a
	ld [address], hl
	
address: db 0xbeaf