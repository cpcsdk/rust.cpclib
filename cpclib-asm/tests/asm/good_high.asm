  LIST
  org 0x1234

VAR1 equ 0x1234
VAR2 equ VAR1/256
VAR3 equ >VAR1
VAR4 equ VAR2*256
VAR5 equ VAR3*256
VAR6 equ VAR3*256 + 0x20

    assert VAR2 == 0x12
    assert VAR3 == 0x12
    assert VAR4 == 0x1200
    assert VAR5 == 0x1200
    assert VAR6 == 0x1220


DICO100                         ; must be 0x1234
PFO100_try1 equ DICO100/256     ; must be 0x12
PFO100_try2 equ >DICO100        ; must be 0x12
PFO100_try3 equ 0x12            ; must be 0x12
NBR_REG equ 13                  ; must be 13


    assert DICO100 == VAR1
    assert PFO100_try1 == VAR2
    assert PFO100_try2 == VAR3
    assert PFO100_try3 == 0x12

    
    assert PFO100_try1+NBR_REG  == 0x12+13
    assert PFO100_try2+NBR_REG  == 0x12+13
    assert PFO100_try3+NBR_REG  == 0x12+13
    assert 0x12+13== 0x12+13

    CP PFO100_try1+NBR_REG      ; must be 0x12+13
    CP PFO100_try2+NBR_REG      ; must be 0x12+13
    CP PFO100_try3+NBR_REG      ; must be 0x12+13
    CP 0x12+13                  ; must be 0x12+13

