    ; Set up a unique value in various banks
    BANK 0xc0
    org 0x4000 + 0
    db 0xc0

    BANK 0xc4
    org 0x4000 + 1
    db 0xc4


    BANK 0xc5
    org 0x4000 + 2
    db 0xc5

    BANK 0xc6
    org 0x4000 + 3
    db 0xc6


    BANK 0xc7
    org 0x4000 + 4
    db 0xc7


    BANKSET 0
    assert memory(0x4000 + 0) == 0xC0 

    BANKSET 1
    assert memory(0x4000 + 2) == 0xC5
    assert memory(0x8000 + 3) == 0xC6
    assert memory(0xC000 + 4) == 0xC7