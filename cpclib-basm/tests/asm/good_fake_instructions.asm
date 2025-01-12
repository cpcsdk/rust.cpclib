	ld hl, de
	jp hl
	jp ix
	jp iy
	push hl, de, bc
	pop bc, de, hl

	
	SRL BC
	SRL DE
	SRL HL

	SRA BC 
	SRA DE 
	SRA HL

	SLL BC
	SLL DE
	SLL HL

	SLA BC
	SLA DE
	SLA HL

	RR BC
	RR DE
	RR HL

	RL BC
	RL DE
	RL HL


	RLC BC 
	RLC DE
	RLC HL

	RRC BC 
	RRC DE
	RRC HL