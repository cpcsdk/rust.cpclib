    ; ASMCONTROL directive - print during parsing
    org $4000
    asmcontrol PRINT_PARSE, "Assembling at address: ", $
    
    ld a, 10
    asmcontrol PRINT_ANY_PASS, "Value loaded: ", 10
    ret
