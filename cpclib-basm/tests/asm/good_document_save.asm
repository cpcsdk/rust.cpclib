    ; SAVE/WRITE directive
    org $4000
    
start:
    ld a, 5
    ret
    
    save "output.bin", start, 10, amsdos
