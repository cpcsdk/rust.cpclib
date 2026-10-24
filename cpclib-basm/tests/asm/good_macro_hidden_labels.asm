; Regression test: repeated macros with forward-referenced @ labels
; should assemble correctly across multiple passes without "already defined"
; errors. Root cause: macro_seed reset at each pass combined with hidden
; symbol persistence in the symbol table across passes.

org 0x4000

MACRO DO_CRTC
    jr @crtc
@crtc:
    nop
ENDM

    DO_CRTC(void)
    DO_CRTC(void)
