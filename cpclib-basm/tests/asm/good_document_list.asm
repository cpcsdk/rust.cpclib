    ; LIST/NOLIST directives
    list        ; Enable listing
    
    org $4000
    ld a, 5
    
    nolist      ; Disable listing
    ld b, 10
    list        ; Re-enable listing
    
    ret
