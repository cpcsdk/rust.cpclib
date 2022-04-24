

    macro KILL_SYSTEM
        di
            ld hl, $c9fb
            ld ($38), hl
            ld sp, $4000
        ei
    endm

    macro WAIT_VSYNC
        ld b, $f5
@loop
        in a, (c)
        rra
        jr nc, @loop
    endm

    macro WAIT_VSYNC_STRICT
        ld b, $f5
@loop1
        in a, (c)
        rra
        jr c, @loop1
        WAIT_VSYNC (void)
    endm

    macro SET_CRTC reg, value
        ld bc, $bc00 + {reg} : out (c), c
        ld bc, $bd00 + {value} : out (c), c
    endm

BACKGROUND equ $54

    struct RASTER_BAR

ink1        db BACKGROUND ; space for ink1 of raster
ink2        db BACKGROUND
ink3        db BACKGROUND
ink4        db BACKGROUND
ink5        db BACKGROUND ; space for ink5 of raster

    endstruct
    
RASTER_HEIGHT equ RASTER_BAR

    org 0x4000

    KILL_SYSTEM (void)
    SET_CRTC 6, 0

    di
    WAIT_VSYNC_STRICT (void)
    
    call frame_loop
    jr $

frame_loop
        WAIT_VSYNC (void)


        ld b, 30
    .loop
            nop 64 - (duration(djnz .loop) + 1) ; duration of djnz is the one with no jump
        djnz .loop

        call show_raster
    
    jp frame_loop


show_raster

    ; manage the sine wave for the vertical position
    nop 15  
    ld hl, sine_curve
.curve_position equ $-2
    ld b, (hl)
    inc l
    ld (.curve_position), hl
.loop
    nop 64 - (duration(djnz .loop) + 1) ; duration of djnz is the one with no jump
    djnz .loop

    ; really display the raster bar
    ld bc, $7f10
    ld hl, raster_table.bar1
    out (c), c
    
    repeat RASTER_HEIGHT, loop

        ticker start @raster_line_duration
            ld a, (hl)
            out (c), a

            if {loop} != RASTER_HEIGHT
                inc hl
            endif
        ticker stop

        nop 64-@raster_line_duration

    endr
    ld a, BACKGROUND
    out (c), a

    ret

raster_table
.bar1
    RASTER_BAR $44, $55, $5C, $55, $44


    function curve_value idx, height
        return height/2*cos(idx*360/256) + height/2 + 1
    endf

    align 256
sine_curve
.height equ min(255, 312 - 30 - 8)

    repeat 256, i
        val set curve_value({i}, .height)
        print {hex}$, "=", val
        db val
    endr





    
