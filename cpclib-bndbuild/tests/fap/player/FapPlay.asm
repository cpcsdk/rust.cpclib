;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                                               ;;
;;                                        AY PROGRAMMING                                         ;;
;;                                                                                               ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

CurrentPlayerBufferLow = $+1
CurrentPlayerBufferHigh = $+2
        ld	hl, #0000
        ld	a, l
        inc	a
Reloc1 = $+1
        ld	(CurrentPlayerBufferLow), a
        exx
        ld	bc, #C680
        exx
NrRegistersToPlay = $+1
        ld	bc, #F400       ; Max number of registers to play is written in the C register by the init code.
        ld	de, #0201

        ;
        ; Write to register 0
        ;
        WriteToPSGRegSkip	0, e
        inc	h

        ;
        ; Write to register 2
        ;
        WriteToPSGRegSkip       d, e
        inc	h

        ;
        ; Write to register 1
        ;
        inc     d
        ld	a, (hl)
        dec     l
        cp	(hl)
Reloc2 = $+1
        jp	z, SkipR1_3
        WriteToPSGReg   e

        ;
        ; Write to register 3
        ;
        rra
        rra
        rra
        rra
        WriteToPSGReg	d
SkipR1_3Return:
        inc     l
        inc	h

        ;
        ; Write to register 4
        ;
        inc	d
        WriteToPSGRegSkip	d, e
        inc	h

        ;
        ; Write to register 5
        ;
        inc     d
        ld	a, (hl)
        inc     h
        bit	5, (hl)                 ; Check if we have to program register 5.
        jr	nz, SkipR5
        WriteToPSGReg	d
SkipR5:

        ;
        ; Write to register 13
        ;
        rra
        rra
        rra
        rra
        bit	6, (hl)                 ; Check if we have to program register 13.
        ld	e, 13
        jr	nz, SkipRegister13
        WriteToPSGReg	e
SkipRegister13

        ;
        ; Write to register 6
        ;
        inc	d
        ld	a, (hl)
        bit	7, a
        jr	nz, SkipRegister6      
        WriteToPSGReg	d
SkipRegister6:
        inc	h

        ;
        ; Write to register 7
        ;
        inc	d
        WriteToPSGRegSkip	d, b
        inc     h

        ;
        ; Write to register 8
        ;
        inc	d
        WriteToPSGRegSkip	d, b
        inc	h

        ;
        ; Write to register 9
        ;
        inc	d
        WriteToPSGRegSkip	d, b
        inc	h

        ;
        ; Write to register 10
        ;
        inc	d
        WriteToPSGRegSkip	d, b
        inc	h
        
        ;
        ; Write to register 11
        ;
        inc	d
        WriteToPSGRegSkip	d, 1

SkipR12OverwriteJR:
        ; Playing R12 is very uncommon. No effort has been made to make this case efficient.
        jr      PlayR12Trampoline

ReturnFromPlayR12:
        jr	z, SkipDecrunchTrampoline2
Reloc3 = $+1
        ld	(NrValuesToDecrunch), a

        ex	de, hl  ; Protect HL from next modification


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                                               ;;
;;                                       AY DATA DECRUNCH                                        ;;
;;                                                                                               ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;


        ;
        ; Move to the next decrunch buffer and handle buffer loop.
        ;
DecrunchEntryPoint:
ReLoadDecrunchSavedState  equ	$ + 1
ReLoadDecrunchSavedStatehigh  equ	$ + 2
        ld	hl, #0000
        ld	a, l
DecrunchStateLoopValue = $+1
        cp	0       ; The loop value is written here by the init code.
        jr	nz, SkipBufferReset
        xor	a
SkipBufferReset:
        ld	l, a
        ld	sp, hl
        ld	a, e    ; Backup current position of the player in the decrunched buffer
        pop	de      ; d = restart if not null       e = Lower byte of source address if restart copy from windows. Undef otherwise.
        pop	bc      ; Current position in decrunch buffer (address bytes are swaped : B=low address byte / C = High address byte)
        pop	hl      ; Current position in crunched data buffer
Reloc4 = $+2
        ld	(ReLoadDecrunchSavedState), sp
        sub	b       ; Compute distance between player read position and current position in decrunch buffer.
        cp	28      ; Leave a security gap between the current decrunch position and the player position.
        jr	c, SkipDecrunchTrampoline
        
        ld	a, h
SwitchResToSet=$+1
        res	7, h
        ld	sp, hl          ; Load current position in decrunch buffer

        ld	h, c
        ld	l, b

        ; SP = current position in decrunch source buffer
        ; HL = current position in decrunch destination buffer
        ; DE = 
        ; BC = B: C: number of values to decrunch
        ; ly = number of markers decoded

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;
        ;;      Decrunch buffers start
        ;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

NrValuesToDecrunch = $+1
        ld	c, 220
NrDecrunchLoop = $+2
        ld	ly, 50
        inc	d
        dec	d
        jr	nz, RestartPausedDecrunch

        ;
        ; Load a new marker
        ;
FetchNewCrunchMarker:
        pop	de

        ld	a, #1F
        cp      e

        jr	nc, CopyLiteral         ; A < 1F --> Copy literals
                                        ; A > 1F --> Copy from dictionnary

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;
        ;;      Copy from dictionnary
        ;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

CopyFromDict:
        ld	a, e
        sub	#1d
        cp	c
        jr	nc, CopySubStringFromDict

        _UpdateNrCopySlot	(void)                  ; 4 NOPS
        _ComputeCopyFromDictSourceAddr	(void)          ; 5 NOPS

RestartCopyFromDict:
        _CopyFromDictLoop	b                       ; 10 * N - 1 NOPS       - MOD: A, DE, HL + B

        dec	ly
        jr	nz, FetchNewCrunchMarker
Reloc5 = $+1
        jp      ExitMainDecrunchLoop

SkipDecrunchTrampoline2:
        ld	a, 15
Reloc6 = $+1
        jp	SkipDecrunchLoop

SkipDecrunchTrampoline:
Reloc7 = $+1
        jp      SkipDecrunch

PlayR12Trampoline:
Reloc8 = $+1
        jp	PlayR12
        
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;
        ;;      Copy Literal
        ;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

CopyLiteral:
        jr	z, DoFramesLoop
        ld	a, e
        inc	a
        
RestartCopyLiteral:
        cp	c
        jr	nc, CopySubLiteralChain

        _UpdateNrCopySlot	(void)          ; 4 NOPS
        _CopyLiteralLoop	b               ; 2 + 10 * N NOPS

        dec	ly
        jr	nz, FetchNewCrunchMarker
        jr      ExitMainDecrunchLoop

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;
        ;;      Continue paused Decrunch
        ;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

        ; D != 0 --> Restart a paused decrunch
        ;   if bit 7(H) = 1
        ;      Restart Copy Literal     D = remaining length    E = unknown
        ;   else
        ;      Restart Copy From Dict   D = remaining length    E = Lower byte

RestartPausedDecrunch:
        rla             ; if Bit 7 is set -> restart a Copy Literal
SwitchNcToC:
        jr	nc, RestartPausedCopyFromDict
        
        ;
        ; Restart Copy Literal
        ;
        ld	a, d
        dec	sp
        pop	de
        jr	RestartCopyLiteral

RestartPausedCopyFromDict:
        SKIP_NOPS 5

        ld	a, d
        cp	c
        ld	d, h
        jr	nc, RestartSubCopyFromDict
        nop

        _UpdateNrCopySlot	(void)          ; 4 NOPS
        jr	RestartCopyFromDict
RestartSubCopyFromDict:
        _AdjustCopySizeWithRemainingSlots	(void)
        nop
        jr	RestartCopySubStringFromDict      

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;
        ;;      Do Frames loop
        ;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        
DoFramesLoop:
        dec     sp
        exx
        pop	hl
DataBufferReset = $+1
        ld	bc, #0000
        add	hl, bc
        ld	sp, hl
        exx
        nop
        dec	c
        ld	d, c
        jr	z, DecrunchFinalize

        SKIP_NOPS 2

        dec	ly
Reloc9 = $+1
        jp	nz, FetchNewCrunchMarker
        jr      ExitMainDecrunchLoop2

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;
        ;;      Copy sub string and jump to decrunch finalize
        ;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

        ;
        ;       Copy from dictionnary
        ;
CopySubStringFromDict:
        _AdjustCopySizeWithRemainingSlots	(void)        ; 2 NOPS
        _ComputeCopyFromDictSourceAddr	(void)                ; 4 (+1) NOPS

RestartCopySubStringFromDict:
        _CopyFromDictLoop	c                             ; 12 * N NOPS        
        ld	d, b
        ld	a, c

        dec     ly
        jr	z, SaveDecrunchState
        jr      EnterStabilizeLoop

        ;
        ; We have more literal to copy than available copy slots
        ;
CopySubLiteralChain:
        sub     c
        _CopyLiteralLoop	c
        ld	d, a

DecrunchFinalize:
        ld	a, #80

        ;
        ; Decrunch stabilization loop
        ;
        dec     ly
StabilizeLoop:
        jr	z, SaveDecrunchState

        SKIP_NOPS 3
EnterStabilizeLoop:
        ld	b, 4
        djnz    $

        dec	ly
        jr	StabilizeLoop

ExitMainDecrunchLoop:
        nop
ExitMainDecrunchLoop2:
        xor	a
        ld	d, a
        SKIP_NOPS 5
        dec	c
        jr	nz, ExitMainDecrunchLoop

        ;
        ; Write back to memory the current decrunch state.
        ;
SaveDecrunchState:
        ld	b, l    ; Dirty trick!!! BC = LH (backup for latter push) while setting HL to AC.
        ld	l, c
        ld	c, h
        ld	h, a
        add	hl, sp
Reloc10 = $+2
        ld	sp, (ReLoadDecrunchSavedState)
        push	hl      ; Save current position in crunched data buffer
        push	bc      ; Save current position in decrunch buffer
        push	de
DecrunchFinalCode:

        ;
        ; Return to the calling code.
        ;
ReturnAddress = $+1
        jp	#0000

SkipR1_3:
        SKIP_NOPS	3
Reloc11 = $+1
        jp      SkipR1_3Return
        
SkipDecrunch:
Reloc12 = $+1
        ld	a, (NrValuesToDecrunch)
        add	a, 11
        SKIP_NOPS 6
SkipDecrunchLoop:
        SKIP_NOPS 8
        dec	a
        jr	nz, SkipDecrunchLoop
        jr      DecrunchFinalCode

PlayR12:
        ;
        ; Write to register 12
        ;
        inc	h
        inc	d
        ld	a, (hl)
        dec     l
        cp	(hl)
        jr	z, SkipRegister12
        WriteToPSGReg	d
SkipRegister12:
        dec     l
        ld	a, c
        add	a, a
Reloc13 = $+1
        jp	ReturnFromPlayR12
