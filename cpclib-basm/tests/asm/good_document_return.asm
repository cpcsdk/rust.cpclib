    ; RETURN directive in function
    function get_value
        return 42
    endfunction
    
    org $4000
    db get_value()  ; Returns 42
    ret
