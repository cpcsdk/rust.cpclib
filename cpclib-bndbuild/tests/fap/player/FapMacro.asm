;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                                               ;;
;;                                             MACROS                                            ;;
;;                                                                                               ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;
; Waste time using as few bytes as possible.
;
MACRO   SKIP_NOPS	Nops
        if      {Nops}	== 2
                cp	(hl)    ; WASTE TIME WITH FEW BYTES (2 NOPS - 1 BYTE)
        else
                if	{Nops}	== 3
                        jr	$+2     ; Add hl, RR    ; inc (hl)      ; pop hl
                else
                        if	{Nops}	== 5
                                cp	a, (ix) ; WASTE TIME WITH FEW BYTES (5 NOPS - 3 BYTES)
                        else
                                if	{Nops}	== 6
                                        inc	(hl)    ; WASTE TIME WITH FEW BYTES (3 NOPS - 1 BYTE)
                                        dec	(hl)    ; WASTE TIME WITH FEW BYTES (3 NOPS - 1 BYTE)
                                else
                                        if	{Nops}	== 7
                                                inc	(hl)    ; WASTE TIME WITH FEW BYTES (3 NOPS - 1 BYTE)
                                                nop
                                                dec	(hl)    ; WASTE TIME WITH FEW BYTES (3 NOPS - 1 BYTE)
                                        else
                                                if      {Nops}	== 8
                                                        inc	(hl)    ; WASTE TIME WITH FEW BYTES (3 NOPS - 1 BYTE)
                                                        cp      (hl)
                                                        dec	(hl)    ; WASTE TIME WITH FEW BYTES (3 NOPS - 1 BYTE)
                                                endif
                                        endif
                                endif
                        endif
                endif
        endif
MEND

;
; Update the number of remaining "copy slots" from the number of bytes to process in the next copy operation.
;
MACRO   _UpdateNrCopySlot               ; 4 NOPS
        ld	b, a
        ld	a, c
        sub	b
        ld	c, a
MEND

;
; Adjust the number of bytes to process in the next copy operation, regarding the remaining "copy slots".
;
MACRO   _AdjustCopySizeWithRemainingSlots         ; 2 NOPS
        sub	c
        ld	b, a
MEND

;
; Compute the source address of data in the dictionary
;
MACRO   _ComputeCopyFromDictSourceAddr   ; 4 NOPS
        ld	a, d
        sub	l
        cpl
        ld	e, a
        ld	d, h
MEND

;
; Copy string from dictionary
;
MACRO   _CopyFromDictLoop	LoopReg ; 10 * N NOPS - 1
@CopyLoop:
        ld	a, (de)     
        ld	(hl), a
        inc	l
        inc	e
        SKIP_NOPS 2
        dec	{LoopReg}
        jr	nz, @CopyLoop
MEND

;
; Copy literals from crunched data.
;

MACRO   _CopyLiteralLoop   LoopReg ; 2 + 10 * N NOPS
@CopyLoop:
        ld      (hl), d
        inc	l
        SKIP_NOPS 5
        dec     {LoopReg}
        jr      nz, @ContinueLoop
        jr      @ExitLoop
@ContinueLoop:
        pop     de
        ld      (hl), e
        inc	l
        SKIP_NOPS 2
        dec     {LoopReg}
        jr	nz, @CopyLoop
        nop
        dec     sp
@ExitLoop:
MEND

        ;
        ; Write a value in a PSG register
        ;
MACRO   WriteToPSGReg	RegNumber       ; 25 NOPS
        out	(c), {RegNumber}
        dec	c              ; Dec number of registers to play

        exx
        out	(c), 0
        exx

        out	(c), a

        exx
        out	(c), c
        out	(c), b
        exx
MEND

MACRO   WriteToPSGRegSkip	RegNumber, SkipVal
        ld	a, (hl)
        cp	{SkipVal}
        jr	z, @Skip

        WriteToPSGReg {RegNumber}
@Skip:
MEND

MACRO   Write8ToPlayerCodeWithReloc	Offset, Value
        ld	hl, {Offset} - FapPlay
        add	hl, bc
        ld	(hl), {Value}
MEND

MACRO   WriteHLToPlayerCodeWithReloc	Offset
        ld	iy, {Offset} - FapPlay
        add	iy, bc
        ld	(iy + 0), l
        ld	(iy + 1), h
MEND

MACRO   WriteHLToInitCodeWithReloc	Offset
        ld	iy, {Offset} - FapInit
        add	iy, de
        ld	(iy + 0), l
        ld	(iy + 1), h
MEND
