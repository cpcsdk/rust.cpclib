        NR_REGISTERS_TO_DECRUNCH	equ #0C
        
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                                               ;;
;;                                          PLAYER INIT                                          ;;
;;                                                                                               ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

        ;
        ; Params:
        ;       A:  High byte of decrunch buffer
        ;       BC: Address of the player code
        ;       DE: RET address to jump at the end of the player execution.
        ;       HL: Music crunched data buffer
        exa             ; Backup A for later use
        push	de      ; Address to return from the player

        ;
        ;       Get the value of PC (small trick for PIC code). We need it for some PIC code.
        ;
        exx
        ld	de, (#0000)     ; Backup bytes from #0000
        ld	hl, #E9E1       ; Write POP HL; JP (HL)
        ld	(#0000), hl
        call	#0000
RetFromGetPC2:
        ld	(#0000), de     ; Restore bytes

        ; Compute address of FapInit base address
        ld	iy, FapInit - RetFromGetPC2
        ex	de, hl
        add	iy, de
        push	iy              ; Push for future use
        exx

        push	hl      ; Address of crunched data

        ;
        ; Check if data buffer is in high memory. If so we have to modify the player code.
        ; Indeed, the player uses the high bit of the address to store a flag.
        ; So, depending on the address given by the user, we have to invert the logic related to this flag.
        ;
        bit	7, h
        jr	z, DataInLowerMemory

        push    hl
        Write8ToPlayerCodeWithReloc	SwitchResToSet, #FC     ; Switch "res 7, h" to "set 7, h"
        Write8ToPlayerCodeWithReloc	SwitchNcToC, #38        ; switch "jr nc" to "jr c"
        pop	hl

DataInLowerMemory:

        ;
        ; Do player code relocation
        ;
        ld	ix, #0000: add ix, sp      ; Backup SP

        push	bc      ; Pass BC to BC' using push / pop
        exx
        pop     bc

        ld	de, RelocTable - FapInit
        add	iy, de
        ld	sp, iy
        ld	a, (RelocTableEnd - RelocTable) / 2

RelocMainLoop:
        pop     hl
        add	hl, bc
        ld	e, (hl)
        inc	hl
        ld	d, (hl)
        ex	de, hl
        add	hl, bc
        ex	de, hl
        ld	(hl), d
        dec	hl
        ld	(hl), e

        dec	a
        jr	nz, RelocMainLoop

        exx
        ld	sp, ix                  ; Restore SP

        ;
        ; Initialize DataBufferReset in the player code. 
        ;
        WriteHLToPlayerCodeWithReloc	DataBufferReset

        ld	xl, NR_REGISTERS_TO_DECRUNCH

        ;
        ; Load Skip R12 flag
        ;
        ld	a, (hl)
        inc     hl
        or	a
        jr	z, NoSkipR12

        ; Let skip the R12 play.
        dec     xl
        exx
        ld	hl, #8779       ; Overwrite JR to PlayR12 with some instructions
        WriteHLToPlayerCodeWithReloc    SkipR12OverwriteJR
        exx

NoSkipR12:
        ld	a, xl   ; Let N = Number of registers to decrunch
        add	a, a    ; A = 2 * N
        ld	b, a    ; B = 2 * N
        add	a, a    ; A = 4 * N
        add	b       ; A = 6 * N

        exx
        Write8ToPlayerCodeWithReloc	DecrunchStateLoopValue, a
        exx

        ;
        ; Load number of registers to play
        ;
        ld	a, (hl)
        inc	hl

        exx
        Write8ToPlayerCodeWithReloc	NrRegistersToPlay, a
        exa
        Write8ToPlayerCodeWithReloc	CurrentPlayerBufferHigh, a
        exa

        ;
        ; Initialize registers
        ;
        push	bc              ; Backup player base address
        ld	bc, #C680
        exx
        ld	bc, #F400
        ld	de, #000E
InitRegisterLoop:
        ld	a, (hl)
        inc	hl
        WriteToPSGReg	d
        inc	d
        dec	e
        jr	nz, InitRegisterLoop

        ;
        ;       Compute the address of the decrunch state array. The array is located right after the decrunch buffers.
        ;
        exx
        pop	bc              ; Restore player base address
        exa
        add	a, xl
        Write8ToPlayerCodeWithReloc	ReLoadDecrunchSavedStateHigh, a
        exx

        ld	d, a
        sub	a, xl
        exa
        ld	e, 0            ; DE = address of the decrunch state array

        ;
        ; Initialize decrunch save state array.
        ;
        ld	xh, xl
        pop     bc
InitDecrunchStateLoop:
        ; Write #0000 (restart decrunch flags)
        xor	a
        ld	(de), a
        inc	de
        ld	(de), a
        inc	de

        ; Write initial position in decrunch (dest) buffer.
        exa
        ld	(de), a
        inc     a
        exa
        inc	de
        ld	(de), a
        inc	de

        ; Write initial position in crunched (src) buffer.
        ld	a, (hl)
        add	a, c
        ld	(de), a
        inc	de
        inc	hl
        ld	a, (hl)
        adc	a, b
        ld	(de), a
        inc	de
        inc	hl

        dec     xh
        jr	nz, InitDecrunchStateLoop

        ;
        ;       Update the player return adress to return in the following init code.
        ;
        exx
        pop	de              ; Base address of FapInit
        ld	hl, ReturnFromDecrunchCodeToInitCode - FapInit
        add	hl, de
        WriteHLToPlayerCodeWithReloc	ReturnAddress

        ;
        ;       Compute address of DecrunchEntryPoint
        ;
        ld	hl, DecrunchEntryPoint - FapPlay
        add	hl, bc
        WriteHLToInitCodeWithReloc JumpToDecrunchEntryPoint
        ld	hl, sp          ; Backup SP
        exx

        ;
        ; Loop to initialize decrunch buffers
        ;
        ld	xh, xl
InitDecrunchBufferLoop:
        ld	iy, #FFFF
        ld	e, #FF
JumpToDecrunchEntryPoint =$+1
        jp	#0000

ReturnFromDecrunchCodeToInitCode:
        ld	sp, #0000
        dec     xh
        jr	nz, InitDecrunchBufferLoop

        exx:    ld sp, hl       ; Restore SP

        Write8ToPlayerCodeWithReloc NrDecrunchLoop, 4

        pop	hl
        WriteHLToPlayerCodeWithReloc	ReturnAddress

        exx

        ret

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                                               ;;
;;                                        RELOCATION TABLE                                       ;;
;;                                                                                               ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

RelocTable:
        dw	Reloc1 - FapPlay
        dw	Reloc2 - FapPlay
        dw	Reloc3 - FapPlay
        dw	Reloc4 - FapPlay
        dw	Reloc5 - FapPlay
        dw	Reloc6 - FapPlay
        dw	Reloc7 - FapPlay
        dw	Reloc8 - FapPlay
        dw	Reloc9 - FapPlay
        dw	Reloc10 - FapPlay
        dw	Reloc11 - FapPlay
        dw	Reloc12 - FapPlay
        dw	Reloc13 - FapPlay
RelocTableEnd:
