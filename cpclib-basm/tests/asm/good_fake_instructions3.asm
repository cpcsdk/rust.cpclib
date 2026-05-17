	LD BC,(IX+3) ; lecture 16 bits d'un registre à l'adresse indirecte
	LD DE,(IX+3)
	LD HL,(IX+3)
	LD (IX+3),BC
	LD (IX+3),DE
	LD (IX+3),HL
	LD BC,(IY+3) ; version avec IY
	LD DE,(IY+3)
	LD HL,(IY+3)
	LD (IY+3),BC
	LD (IY+3),DE
	LD (IY+3),HL