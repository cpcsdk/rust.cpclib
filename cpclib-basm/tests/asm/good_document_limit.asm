    ; LIMIT directive
    org $4000
    limit $4100  ; Fail if code exceeds this address
    
    ; This code fits within the limit
    ld a, 5
    ld b, 10
    ret
