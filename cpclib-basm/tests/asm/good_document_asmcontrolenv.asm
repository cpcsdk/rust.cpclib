    ; ASMCONTROLENV example - limit number of passes
    org $4000
    
    asmcontrolenv SET_MAX_NB_OF_PASSES = 2
        ; Code that must assemble in 2 passes or fewer
        nop
        ld a, 5
    endasmcontrolenv
    
    ret
