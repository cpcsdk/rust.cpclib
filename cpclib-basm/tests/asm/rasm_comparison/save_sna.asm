	; cargo run -- save_sna.asm  --nochunk BRKS --sna -o basm.sna
	; to be compared with 
	; rasm -sb -ss  save_sna.asm -o rasm


	BUILDSNA
	BANKSET 0

	org 0x1234
BINARY_START
	run $
START
	BREAKPOINT
	jp $
	BREAKPOINT
BINARY_STOP