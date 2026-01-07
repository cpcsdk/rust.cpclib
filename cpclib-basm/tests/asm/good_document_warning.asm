    ; Test WARNING directive
    ; WARNING emits a warning message without stopping assembly
    
    org 0x4000
    
    ; Basic warning
    WARNING "This is a test warning"
    
    ; Warning with expressions
value = 42
    WARNING "Value is ", value
    
    ; Warning with formatted expressions
    WARNING "Value in hex: ", {HEX} value
    WARNING "Value in binary: ", {BIN} value
    
    ; Conditional warning
    IF value > 40
        WARNING "Value exceeds threshold"
    ENDIF
    
    ; Warning vs FAIL comparison
    ; WARNING continues assembly
    WARNING "This is just a warning"
    ld a, 1    ; This instruction will be assembled
    
    ; FAIL would stop assembly
    ; FAIL "This would stop assembly"    ; Commented out
    
    ; Test multiple warnings
    REPEAT 3, i
        WARNING "Iteration ", {i}
    ENDREPEAT
    
    ld b, 2
