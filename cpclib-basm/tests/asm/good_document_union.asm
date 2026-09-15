    ; UNION example - reusing the same scratch RAM for two purposes that
    ; never happen at the same time: a raw loading buffer while a level is
    ; being decompressed, then a shaped sprite once loading is done.
    STRUCT Sprite
        x db 0
        y db 0
        frame db 0
    ENDSTRUCT

    org $4000

    UNION
        loading_buffer: ds 16
    NEXTU
        spr: Sprite(void)
    ENDU

    ; loading_buffer and spr alias the same address
    assert loading_buffer == spr

    ; the union reserved the MAX of its members' sizes (16), not their sum
    assert $ == $4000 + 16

    ret
