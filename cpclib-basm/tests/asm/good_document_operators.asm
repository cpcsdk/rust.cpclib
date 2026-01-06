    ; Operators example
    org $4000
    
    value = (5 + 3) * 2       ; = 16
    mask = $FF & %00001111    ; = $0F
    shifted = 1 << 4          ; = 16
    high_byte = high($1234)   ; = $12
    low_byte = low($1234)     ; = $34
    
    ret