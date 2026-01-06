    ; PROTECT directive
    org $4000
    
    ; Protect memory from $4000 to $4100
    protect $4000, $4100
    
    ; This will fail because we're in protected memory
    ; Uncomment to test:
    ; db 1, 2, 3
    
    ; Move outside protected area
    org $4200
    db 1, 2, 3
    ret
