;;
; Test the generation of a binary file embeded in a basic file

basic_prog
    LOCOMOTIVE {binary_prog}
    HIDE_LINES 40
10 ' Time to test the ability
20 ' to inject birary files
30 ' inside basic programs !!
40 call {binary_prog}
    ENDLOCOMOTIVE

binary_prog
    ld hl, my_text
.loop
    ld a, (hl) : inc hl
    or a : jp z, .finished
    call 0xbb5a
    jr .loop
.finished
    jr $

my_text
    db "Hello world from basic"
    db 0
