    macro add_to_a value
        add a, {value}
    endm
    
    ld a, 5
    add_to_a(10)
    add_to_a(3)
    
