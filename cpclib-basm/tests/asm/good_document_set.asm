    ; =/SET directive
    counter = 10
    counter = counter + 1  ; Can reassign
    counter = counter * 2
    
    db counter  ; Should be 22
    ret
