; @-prefixed labels are local to each MACRO call / REPEAT iteration - the
; same body can define one without ever colliding with another call's or
; iteration's own @label of the same name.
org 0x4000

MACRO WAIT_LOOP
@loop:
    djnz @loop
ENDM

    ld b, 5
    WAIT_LOOP(void)     ; expands to its own @loop, e.g. label ".__hidden__1__loop"
    ld b, 3
    WAIT_LOOP(void)     ; a second, independent @loop - no "already defined" error

; Same mechanism inside REPEAT - each iteration gets its own @loop.
REPEAT 3
@loop:
    djnz @loop
ENDREPEAT
