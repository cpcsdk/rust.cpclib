    ; WORD/DW/DEFW directive - define 16-bit words
    org $4000
    
    dw $1234
    defw $ABCD
    word $5678, $9ABC
    
    ret
