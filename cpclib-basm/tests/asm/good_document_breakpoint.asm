    ; BREAKPOINT directive
    org $4000
    
    ld a, 5
    breakpoint  ; Debugger will stop here
    ld b, 10
    
    ret
