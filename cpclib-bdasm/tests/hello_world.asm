; Z80 Hello World for Amstrad CPC
; This program prints "Hello World 323!" to the screen

    PrintChar     equ &BB5A   

    org &1200
    run $

    ld hl,Message            ;Address of string
    Call PrintString        ;Show String to screen

    ret                ;Finished Hello World

PrintString:
    ld a,(hl)    ;Print a '255' terminated string
    cp 255
    ret z
    inc hl
    call PrintChar
    jr PrintString

Message: db 'Hello World 323!',255

NewLine:
    ld a,13        ;Carriage return
    call PrintChar
    ld a,10        ;Line Feed
    jp PrintChar
