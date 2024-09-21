    ORG	#3000      
    RUN	$

    ;BuffSize	equ #B42		; Size of replay buffer given by the cruncher.
    PlayerSize	equ 609			; Size of the FAP player code

    FapInit	equ #C000       	; Address of the player initialization code.
    FapBuff	equ #4000       	; Address of the decrunch buffers (low order byte MUST BE 0).
    FapPlay	equ FapBuff+BuffSize  	; Address of the player code. Right after the decrunch buffer.
    FapData	equ FapPlay+PlayerSize	; Address of the music data. Right after the player routine.

    ;
    ; You known the story ;)
    ;
    ld	hl, #C9FB
    ld	(#38), hl

    ;
    ; Initialize the player.
    ; Once the player is initialized, you can overwrite the init code if you need some extra memory.
    ;
    ld	a, hi(FapBuff)	; High byte of the decrunch buffer address.
    ld	bc, FapPlay     ; Address of the player binary.
    ld	de, ReturnAddr  ; Address to jump after playing a song frame.
    ld	hl, FapData     ; Address of song data.
    call    FapInit

    ;
    ; Main loop
    ;
MainLoop:
    ld	b, #F5
    in	a, (c)
    rra
    jr	nc, MainLoop

    di			; Prevent interrupt apocalypse
    ld	(RestoreSp), sp	; Save our precious stack-pointer
    jp	FapPlay		; Jump into the replay-routine

ReturnAddr:		; Return address the replay-routine will jump back to

RestoreSp = $+1
    ld	sp, 0		; Restore our precious stack-pointer
    ei			; We may enable the maskable interrupts again

    halt		; Wait to make sure the VBL is over.
    halt

    jp	MainLoop

    ;
    ; Load files
    ;
    org	FapInit: incbin "Build/fap-init.bin"
    org	FapPlay: incbin "Build/fap-play.bin"
    org	FapData: incbin MUSIC