    ; DEFSECTION is an alias for RANGE - Syntax: DEFSECTION start, stop, name
    defsection $4000, $8000, code_section
    defsection $8000, $9000, data_section
    
    ; Now use the sections
    section code_section
    ld a, 5
    ret
    
    section data_section
    db 1, 2, 3, 4, 5
