; dummy example that generate a snapshot
; basm snapshot.asm --sna -o test.sna

	org 0x4000
	jp $
	RUN $
	
	di
	ld hl, 0xc9fb
	ld (0x38), hl
	ei

loop
	ld b, 0xf5 : in a, (c) : rra : jr nc, $-3
	
	ld bc, 0x7f10 : out (c), c
	ld a, 0x54 : out(c), a

	halt : halt : halt

	ld a, 0x45 : out(c), a
	jp loop

	