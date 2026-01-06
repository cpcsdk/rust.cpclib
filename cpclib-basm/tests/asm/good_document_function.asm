    FUNCTION square, x
        RETURN {x} * {x}
    ENDFUNCTION
    
    org $4000
    ; Use the function
    db square(5)
    db square(10)
    
    ; Verify with assertions
    assert square(5) == 25
    assert square(10) == 100
    assert memory($4000) == 25
    assert memory($4001) == 100
