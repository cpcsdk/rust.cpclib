; Regression test: seed divergence between passes due to conditional code.
;
; When an IF block adds an extra macro expansion on pass 2 (because a symbol
; was defined on pass 1), the same macro call gets a different seed on each
; pass. Hidden @ labels defined with seed N on pass 1 are referenced with
; seed M on pass 2, causing "Unknown symbol" errors.

org 0x4000

MACRO DEF_CRTC3
@crtc3:
    nop
ENDM

MACRO USE_CRTC3
    jr z,@crtc3
@crtc3:
    nop
ENDM

; On pass 1, crtc_seen is not defined, so the IF is skipped.
; On pass 2, crtc_seen IS defined (from pass 1), so the IF executes an
; extra macro expansion, shifting all subsequent seeds by 1.
IF crtc_seen
    DEF_CRTC3(void)
ENDIF

DEF_CRTC3(void)
USE_CRTC3(void)

crtc_seen = 1
