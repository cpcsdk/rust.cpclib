    ; Test PHASE/DEPHASE directives
    ; PHASE allows assembling code at one address but positioning it at another
    
    org 0x4000
    
    ; Code assembled at 0x4000
    ld a, 1
    ld b, 2
    
    ; Now assemble code for execution at 0x8000, but still place it contiguously
    PHASE 0x8000
    ld hl, relocatedCode    ; This will reference 0x8000+offset
    call relocatedCode
relocatedCode:
    ld a, 42
    ret
    DEPHASE
    
    ; Back to normal addressing
    ld c, 3
    
    ; Test RORG/REND (alternative names)
    RORG 0x9000
    ld de, 0x1234
    REND
    
    ; Verify $ is correct after PHASE/DEPHASE
    ; PHASE code takes 5 bytes (3 for ld hl + 2 for call), + 2 for ld a,42 + 1 for ret
    ; Plus 2 for initial ld a,1 and 2 for ld b,2 and 1 for ld c,3 and 3 for RORG section
    assert $ == 0x4000 + 2 + 2 + (3 + 3 + 2 + 1) + 2 + 3
    ; Should be 0x4012
