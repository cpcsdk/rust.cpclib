    ; ORG directive - set assembly address
    org $4000
    ld a, 5
    ret
    
    ; Move to another address
    org $8000
    ld b, 10
    ret
