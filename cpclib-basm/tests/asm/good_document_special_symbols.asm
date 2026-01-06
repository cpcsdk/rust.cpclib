    ; Special symbols example
    org $4000
    
start:
    ld a, ($ + 5)  ; Reference current address + 5
    db $ - $$      ; Offset from section start
    
    ret