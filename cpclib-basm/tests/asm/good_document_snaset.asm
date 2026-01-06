    ; Snapshot generation example
    buildsna 
    
    org $8000
    ld a, 42
    ret

    ; Set snapshot registers (must use Z80_ prefix)
    snaset Z80_PC, $8000
    snaset Z80_SP, $C000
    snaset Z80_AF, $0000
    snaset Z80_AFX, $0000  ; Alternate AF register

    ; Set CRTC (display controller) registers
    snaset CRTC_SEL, 1      ; Selected CRTC register index
    snaset CRTC_REG:0, 63   ; CRTC register 0 (Horizontal Total)
    snaset CRTC_REG:1, 40   ; CRTC register 1 (Horizontal Displayed)
    snaset CRTC_HCC, 10     ; Horizontal character counter
    snaset CRTC_CLC, 5      ; Character-line counter
    snaset CRTC_TYPE, 0     ; CRTC type: 0 = HD6845S/UM6845
    snaset CRTC_STATE, 255  ; CRTC internal state

    ; Set Gate Array palette (indexed flags)
    snaset GA_PAL:0, $54    ; Palette entry 0
    snaset GA_PAL:1, $44    ; Palette entry 1

    ; Build the snapshot
