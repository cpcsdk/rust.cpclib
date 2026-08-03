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

	; https://roudoudou.com/AmstradCPC/programmationAssembleurVracSuperInstructions.html

	RST Z,#38
	RST NZ,#38
	RST C,#38
	RST NC,#38

	; JQ: try a JR encoding, fall back to a JP encoding if the target is
	; out of relative-jump range - no reachability analysis beyond that
	jq near_target
near_target:
	nop
	jq c, near_target





	

	ld hl, sp
