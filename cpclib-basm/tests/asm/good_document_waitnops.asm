    ; WAITNOPS directive
    org $4000
    
    ld a, 5
    waitnops 100  ; Wait for 100 NOPs
    ld b, 10
    
    ret
