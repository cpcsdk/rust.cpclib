    ; Chained conditionals with ELSEIF
    value = 2
    
    if value == 1
        db 1
    elseif value == 2
        db 2
    elseif value == 3
        db 3
    else
        db 0
    endif
