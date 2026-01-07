    ; Test OUTPUT directive
    ; OUTPUT sets the output filename for the assembled code
    
    ; Set output file
    OUTPUT "testoutput.bin"
    
    org 0x4000
    
    ld a, 1
    ld b, 2
    ld c, 3
    
    ; The assembled code should go to testoutput.bin
    ; instead of the default output file
