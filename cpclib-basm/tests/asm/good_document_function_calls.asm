    ; Function calls example
    org $4000
    
    ; Functions in expressions
    ld a, high($ABCD)
    ld b, low($ABCD)
    ld c, max(10, 20, 30)
    
    ret