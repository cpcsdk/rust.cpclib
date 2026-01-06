    ; IFNOT directive
    value = 0
    
    ifnot value
        ld a, 1  ; Executed because value is 0 (false)
    endif
    
    ret
