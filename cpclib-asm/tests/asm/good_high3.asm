 
    org 0x1234
DICO100  equ $                  ; must be 0x1234


PFO100_try1 equ DICO100/256     ; must be 0x12
    assert PFO100_try1 == 0x12 ; the assertion seems to aknowledge that it is ok

    ld a, 0x12          ; Should be: 0x3e 0x12 Is :  0x3e 0x12 
    ld a, PFO100_try1   ; Should be: 0x3e 0x12 Is :  0x3e 0x00 
    ld a, DICO100/256   ; Should be: 0x3e 0x12 Is :  0x3e 0x00 
    ld a, 0x1234/256    ; Should be: 0x3e 0x12 Is :  0x3e 0x12 
