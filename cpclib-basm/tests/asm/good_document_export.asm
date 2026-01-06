    ; EXPORT/NOEXPORT directive
    export start
    export my_data
    
    org $4000
start:
    ld a, 5
    
noexport
my_private_label:
    nop
    
    export
my_data:
    db 1, 2, 3
    
    ret
