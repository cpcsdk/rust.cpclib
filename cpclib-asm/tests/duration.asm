
;;
; Checks that the duration(OPCODE) function properly returns the number of nops of a the given opcode
; TODO decide what to do with instructions that rely on flags for a conditional execution


	assert duration(pop de) == duration(pop af)
	assert duration(pop de) == 3
	assert duration(nop) == 1
	assert duration(ld a, VAR) == 2
	assert duration(nop) == 1
	
	defs 64 - duration(out (c), a) + duration(inc l) + duration(ld a, (hl))