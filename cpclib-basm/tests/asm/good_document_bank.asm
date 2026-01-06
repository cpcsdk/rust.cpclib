    ; BANK/BANKSET directive
    bank        ; Enter bank mode (outside snapshot)
	assert $ == 0
    db 1, 2, 3, 4, 5
    
    bank 0xc7      ; Switch to bank 0xc7
	org 0x4000   ; BUG there is currently a bug in basm regarding ORG in banked mode
    assert $ == 0x4000
    db 6, 7, 8, 9, 10
