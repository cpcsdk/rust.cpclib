    ORG	#3000      
    RUN	$


    FapInit	equ #C000       	; Address of the player initialization code.
    FapBuff	equ #4000       	; Address of the decrunch buffers (low order byte MUST BE 0).
    FapPlay	equ FapBuff+BuffSize  	; Address of the player code. Right after the decrunch buffer.
    ; FapData is automatically computed by the assembler

    ;
    ; You known the story ;)
    ;
    ld	hl, #C9FB
    ld	(#38), hl
    ld sp, $

		ld bc, 0xbc00+1 : out (c), c
		ld bc, 0xbd00+0 : out (c), c
    ;
    ; Initialize the player.
    ; Once the player is initialized, you can overwrite the init code if you need some extra memory.
    ;
    ld	a, hi(FapBuff)	; High byte of the decrunch buffer address.
    ld	bc, FapPlay     ; Address of the player binary.
    ld	de, ReturnAddr  ; Address to jump after playing a song frame.
    ld	hl, FapData     ; Address of song data.
    di
    call    FapInit
    ei
    
    ;
    ; Main loop
    ;
MainLoop:
    ld	b, #F5
    in	a, (c)
    rra
    jr	nc, MainLoop

    halt		; Wait to make sure the VBL is over.
    halt
    
    di			; Prevent interrupt apocalypse
    ld	(RestoreSp), sp	; Save our precious stack-pointer

    		ld bc,#7f10		; Border 
		ld a,#4c
		out (c),c		; select border
		out (c),a		; in red

    jp	FapPlay		; Jump into the replay-routine

ReturnAddr:		; Return address the replay-routine will jump back to

RestoreSp = $+1
    ld	sp, 0		; Restore our precious stack-pointer

		ld bc,#7f54
		out (c),c

    ei			; We may enable the maskable interrupts again



    jp	MainLoop

    ;
    ; Load files
    ;
    org	FapInit: incbin FAP_INIT_PATH
    org	FapPlay: incbin FAP_PLAY_PATH
    PlayerSize	equ $ - FapPlay


    FapData: incbin MUSIC