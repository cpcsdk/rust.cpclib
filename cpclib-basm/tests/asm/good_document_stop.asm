    ; STOP/END directive
    org $4000
    
    ld a, 5
    ret
    
    stop  ; Assembly stops here
    
    ; This code is never assembled
    ld b, 10
