    ; PAUSE directive example
    ; Note: PAUSE may halt assembly for user interaction
    ; Commented out to allow automatic testing
    
    org $4000
    ld a, 5
    
    ; pause
    
    ld b, 10
    ret
