; Same source than high, without the assertions
; in order to check if binaries could be ok

  org 0x1234


DICO100                         ; must be 0x1234
PFO100_try1 equ DICO100/256     ; must be 0x12
PFO100_try2 equ >DICO100        ; must be 0x12
PFO100_try3 equ 0x12            ; must be 0x12
NBR_REG equ 13                  ; must be 13

    CP PFO100_try1+NBR_REG      ; must be 0x1f
    CP PFO100_try2+NBR_REG      ; must be 0x1f
    CP PFO100_try3+NBR_REG      ; must be 0x1f
    CP 0x12+13                  ; must be 0x1f


