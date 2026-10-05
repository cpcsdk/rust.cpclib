; Song-info printer shared by the AKG and SID player harnesses - pasted into
; each by `music_run.rs` (`{{INFO_PRINT_CODE}}`), so it only uses syntax both
; basm and rasm accept. The surrounding harness must define:
;   InfoText - rows of zero-terminated ASCII (32..=126), the whole list ended
;              by a single 255 byte (see `info_text_db` in music_run.rs)
;   FontBuf  - 96*8 bytes of free RAM, *not* part of the saved binary
;
; The player harnesses kill the firmware, so there is no TXT OUTPUT (&BB5A):
; text is drawn in mode 2 as 8x8 sprites, with the glyphs read from the font
; in the lower ROM (&3800 + 8*char) as 4k intros do. The lower ROM hides
; &0000-&3FFF - where the harness itself lives - so the glyphs are first
; copied to FontBuf by a tiny stub run from the 48 spare bytes
; that follow the 80 bytes of the first screen row group (&C7D0), the only
; RAM that is neither displayed nor used by the harness.
;
; Call with interrupts disabled and a valid stack; returns with both ROMs
; disabled and a mode 2 white-on-black screen.
InfoShow
        ld bc,#7f8e             ; mode 2, ROMs off
        out (c),c
        ld bc,#7f00             ; pen 0 = black
        out (c),c
        ld bc,#7f54
        out (c),c
        ld bc,#7f01             ; pen 1 = bright white
        out (c),c
        ld bc,#7f4b
        out (c),c

        ld bc,#bc01             ; R1: 40 columns displayed (the players do not
        out (c),c               ; need a blank screen, this makes it visible)
        ld bc,#bd28
        out (c),c
        ld bc,#bc0c             ; R12/R13: screen at &C000
        out (c),c
        ld bc,#bd30
        out (c),c
        ld bc,#bc0d
        out (c),c
        ld bc,#bd00
        out (c),c

        ld hl,#c000             ; clear the screen
        ld de,#c001
        ld bc,#3fff
        ld (hl),0
        ldir

        ld hl,InfoStub          ; install the font-copy stub, and run it
        ld de,#c7d0
        ld bc,InfoStubEnd - InfoStub
        ldir
        jp #c7d0

; Executed at &C7D0, with the lower ROM mapped in. It is assembled here, in
; place, and merely copied: that is only valid because it is position
; independent - no JR/DJNZ, no label of its own, only absolute addresses that
; do not move (the ROM font, FontBuf, InfoAfterFont) - so keep it that way.
InfoStub
        ld bc,#7f8a             ; mode 2, lower ROM on
        out (c),c
        ld hl,#3800 + 32*8      ; glyph of ' ' in the ROM font
        ld de,FontBuf
        ld bc,96*8              ; chars 32..=127
        ldir
        ld bc,#7f8e             ; ROMs off again
        out (c),c
        jp InfoAfterFont
InfoStubEnd

InfoAfterFont
        ld hl,InfoText
        ld de,#c000 + 80 + 1    ; second text row, second column
InfoLine
        ld a,(hl)
        cp 255
        ret z
        push de
InfoChar
        ld a,(hl)
        inc hl
        or a
        jr z,InfoEol
        sub 32
        push hl
        ld l,a
        ld h,0
        add hl,hl
        add hl,hl
        add hl,hl
        ld bc,FontBuf
        add hl,bc               ; hl = glyph
        push de
        ld b,8
InfoGlyph
        ld a,(hl)
        ld (de),a
        inc hl
        ld a,d
        add a,8                 ; next scanline of the character
        ld d,a
        djnz InfoGlyph
        pop de
        inc de                  ; next column
        pop hl
        jr InfoChar
InfoEol
        pop de                  ; start of this row
        push hl                 ; text pointer
        ld hl,80
        add hl,de
        ex de,hl                ; de = start of the next row
        pop hl
        jr InfoLine
