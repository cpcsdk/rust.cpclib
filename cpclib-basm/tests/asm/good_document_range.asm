    ; RANGE/SECTION directive - Syntax: RANGE start, stop, name
    range $4000, $5000, code_area
    range $8000, $8800, data_area
    
    section code_area
    ld a, 5
    ret
    
    section data_area
    db 1, 2, 3, 4, 5
