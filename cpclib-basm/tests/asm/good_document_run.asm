    ; RUN directive
    org $4000
    
    ld a, 5
    ret
    
    run $4000  ; Set execution address
