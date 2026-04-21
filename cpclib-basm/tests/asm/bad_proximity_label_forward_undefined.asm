; Test error: using _+ but never defining the next _ label
; This should fail because _+ references a label that never exists

        org 0x4000
        
        nop
        jr _+           ; ERROR: no next _ label defined
        nop
        ret
