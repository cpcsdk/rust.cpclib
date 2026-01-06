    ; INCBIN/BINCLUDE directive
    org $4000
    
    ; Include binary file
    incbin "good_all.bin"
    
    ret
