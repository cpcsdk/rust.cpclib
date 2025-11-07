org 0x3000
run $


    ld hl, 0xc9fb
    ld  (0x38), hl


MainLoop:
    ld  b, 0xf5
    in  a, (c)
    rra
    jr  nc, MainLoop

    halt
    halt

    jp  MainLoop


    incbin "{{MUSIC}}"