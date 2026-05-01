
	ld a, opcode(inc e)
	ld a, opcode(dec e)

	db opcode(out (c), e)