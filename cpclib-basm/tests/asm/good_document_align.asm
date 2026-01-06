    ; ALIGN directive
    org $4000
    
    db 1, 2, 3
    
    align 256   ; Align to next 256-byte boundary
    
    db 4, 5, 6
    ret
