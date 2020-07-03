
;;
; Checks that the opcode(OPCODE) function properly returns the 8bit value of the provided opcode.
; TODO decide what to do with instruction > 8bits
OPCODE_POP_DE equ 0xd1
OPCODE_POP_HL equ 0xe1

	assert opcode(pop de) == OPCODE_POP_DE
	assert opcode(pop hl) == OPCODE_POP_HL
	assert opcode(pop hl) != OPCODE_POP_DE