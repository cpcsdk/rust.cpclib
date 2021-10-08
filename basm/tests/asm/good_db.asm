

	; defb tests
	org 0x200

	defb 1, 2, 3, 4
	defb "hello", ' ', "world"
	defb $, $ ; should generate 2 different values