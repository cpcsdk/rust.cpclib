    ; ASSERT directive
    value = 42
    
    assert value == 42, "Value must be 42"
    assert value > 0
    
    db value
    ret
