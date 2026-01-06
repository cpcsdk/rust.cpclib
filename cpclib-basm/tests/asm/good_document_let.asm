    ; LET directive for explicit variable assignment
    let value = 42
    let address = $4000
    
    ; Use the variables
    org address
    ld a, value
    ret
