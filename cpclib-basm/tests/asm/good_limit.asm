
    org 0x100

    limit 0x102
    print {hex}$ : db 1 ; written in 0x100
    print {hex}$ : db 2 ; written in 0x101
    print {hex}$ : db 3 ; written in 0x102
    ;print {hex}$ : db 4 ; written in 0x103 => must fail
