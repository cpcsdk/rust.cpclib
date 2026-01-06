    ; Define a label
unused_label:
    nop

used_label:
    nop
    
    ; This block is assembled because unused_label is not referenced
    ifnused unused_label
        ld a, 1
    endif
    
    ; This block is NOT assembled because used_label is referenced below
    ifnused used_label
        ld b, 2
    endif
    
    ; Use one of the labels
    jp used_label
