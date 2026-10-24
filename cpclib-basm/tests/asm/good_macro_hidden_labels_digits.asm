; Regression test: repeated macros with forward-referenced @ labels whose
; names end in digits should assemble correctly across multiple passes.

org 0x4000

MACRO DO_CRTC1
    jr @crtc1
@crtc1:
    nop
ENDM

    DO_CRTC1(void)
    DO_CRTC1(void)
