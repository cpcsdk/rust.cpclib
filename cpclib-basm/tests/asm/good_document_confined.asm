    ; CONFINED directive
    org $4000
    
    confined
        ; This data is confined to 256 bytes
        ; Can navigate with INC L without overflow
        db 0, 1, 2, 3, 4, 5, 6, 7, 8, 9
    endconfined
    
    ret
