    ; BREAKPOINT directive
    org $4000

    ld a, 5
    breakpoint  ; Debugger will stop here
    ld b, 10

    ; ACE's form: stops on the write of &55 to the Gate Array's ink register
    ld bc, #7f10
    ld a, #54
loop
    out (c), c
    out (c), a
    inc a
    jr loop
    breakpoint #7f00, type=io, access=write, mask=#ff00, value=#40, valmask=#c0, condition="A = &55", name="ink"
    ; reported without stopping, on every write to buffer (defined further on)
    breakpoint type=mem, access=write, runmode=watch, addr=buffer, name="buffer write"

    ret
buffer
    defb 0
