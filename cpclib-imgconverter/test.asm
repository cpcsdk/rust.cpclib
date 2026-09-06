 ; Linear sprite display routine. HL contains screen address
test_asm
 ; > Handle line 0
 ;   HL already contains the destination address
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 1
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 2
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 3
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x81
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 4
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 5
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 6
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 7
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 8
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 9
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 10
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 11
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 12
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 13
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 14
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 15
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 16
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 17
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 18
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 19
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 20
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 21
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 22
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 23
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 24
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 25
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 26
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 27
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 28
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 29
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 30
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 31
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 32
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 33
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 34
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 35
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 36
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 37
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 38
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 39
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 40
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x21
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 41
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x80
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 42
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 43
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x42
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 44
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 45
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 46
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 47
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 48
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 49
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 50
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 51
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 52
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 53
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x94
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 54
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x16
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x54
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x16
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x94
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 55
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x94
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x34
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x12
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x74
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 56
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x80
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x16
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 57
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x94
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 58
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 59
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x21
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 60
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 61
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x81
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 62
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x34
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 63
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 64
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 65
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 66
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 67
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x58
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 68
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 69
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x24
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 70
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 71
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x21
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 72
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 73
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x12
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 74
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 75
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 76
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 77
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 78
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 79
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 80
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 81
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 82
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 83
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 84
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 85
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 86
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 87
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 88
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x8c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 89
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x58
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x6
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x46
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x89
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x6
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 90
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x9
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x46
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x6
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 91
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x6
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x89
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x4c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 92
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 93
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x8c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xcc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; > Handle line 94
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 95
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; > Handle line 96
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x94
 ; End of line
 ; > Handle line 97
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0x78
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; > Handle line 98
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; End of line
 ; > Handle line 99
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x68
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; > Handle line 100
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 101
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 102
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 103
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 104
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 105
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 106
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 107
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 108
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x40
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 109
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x40
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x34
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 110
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 111
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 112
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 113
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 114
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 115
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 116
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x21
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 117
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa9
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 118
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x34
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 119
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xfc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 120
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x16
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 121
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x7c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 122
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x12
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 123
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x21
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x81
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x68
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 124
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x54
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 125
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x70
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x20
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 126
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x12
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x12
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 127
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x12
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x12
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 128
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x20
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 129
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x2
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x28
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x80
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 130
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x68
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x18
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x21
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x2
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 131
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x52
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x28
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x58
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x58
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x80
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x68
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 132
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x94
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x58
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x74
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x28
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 133
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x74
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x9
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x94
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 134
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x94
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf4
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x30
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 135
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa4
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x42
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 136
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x42
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x9
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x34
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x70
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 137
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x12
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x12
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x94
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 138
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x12
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x42
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xb0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x2
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 139
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x10
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x68
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 140
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x28
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 141
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x30
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x42
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x81
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 142
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x10
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 143
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 144
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x90
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x52
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x50
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x60
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 145
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x60
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x50
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x90
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 146
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 147
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 148
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 149
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 150
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 151
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 152
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 153
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 154
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 155
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x3c
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 156
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 157
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 158
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xf0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 159
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 160
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 161
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x56
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 162
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x56
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 163
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x78
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 164
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x78
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 165
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 166
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x81
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xbc
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 167
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x94
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 168
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x16
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 169
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x68
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x16
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 170
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x29
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0x68
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 171
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x68
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0x29
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 172
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe8
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; > Handle line 173
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; End of line
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xe8
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xa1
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of -1 bytes
	DEC L
 ; No masking here
	LD (HL), 0xc0
 ; > Handle line 174
 ; Compute the address of the next line
	CALL universal_bc26_r1_32
 ; Move of 0 bytes
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xd0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xe0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; Move of 1 bytes
	INC L
 ; No masking here
	LD (HL), 0xc0
 ; End of line
	RET
		IFNDEF universal_bc26_r1_32
universal_bc26_r1_32
	LD A, H
	ADD 0x8
	LD H, A
	AND 0x38
	RET NZ
	LD A, 0x40
	ADD L
	LD L, A
	LD A, 0xc0
	ADC H
	LD H, A
	RES 0x3, H
	RET

	ENDIF

