; Test error: using _- before any _ label is defined
; This should fail because there's no previous _ to reference

        org 0x4000
        
        nop
        jr _-           ; ERROR: no previous _ label exists yet
