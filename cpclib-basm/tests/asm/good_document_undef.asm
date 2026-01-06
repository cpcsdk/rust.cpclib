    ; UNDEF directive
    value = 42
    db value
    
    undef value  ; Undefine the label
    
    value = 100  ; Can redefine now
    db value
    
    ret
