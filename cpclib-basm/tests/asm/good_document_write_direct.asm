    ; WRITE DIRECT directive
    
    write direct -1, -1, 0xc0  ; Write to bank c0
    
    org $4000
    db 1, 2, 3, 4, 5
    ret
