    ; TICKER directive
    org $4000
    
    ticker start timing_var
        ld a, 5
        ld b, 10
        add a, b
    ticker stop
    
    ; timing_var now contains execution time
    db timing_var
    ret
