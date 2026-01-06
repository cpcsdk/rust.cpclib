    ; ENT directive
    org $4000
    
    ld a, 5
    ret
    
    ent $4000  ; Set entry point for AMSDOS
