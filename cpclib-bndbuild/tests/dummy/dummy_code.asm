    org 0x4000



    include "inner://crtc.asm"      ; This file is included in the assembler and contains CRTC constants
    include "inner://ga.asm"        ; This file is included in the assembler and contains GA related functions constants
    include "dummy_logo_conf.asm"   ; This file is generated by the image converter

    print "logo size: ", dummy_logo_conf_WIDTH, " bytes x ", dummy_logo_conf_HEIGHT, " lines"

SCREEN_CRTC_ADDRESS equ 0x3000
SCREEN_MEMORY_ADDRESS equ 0xC000

    macro CRTC_SET_INIT_STATE
CRTC_SET_REG_B = '?'  
CRTC_SET_REG_C = '?'
CRTC_SET_SELECTED_CRTC_REGISTER = '?'  
    endm

    ; Macro example that uses a state to generate the appropriate code.
    ; NOte that this is not the best optimized code, just a quick and dirty one ...
    ; however it gives an idea of what is doable with standard macros (there is nothing fancy here)
    ; or use a function that takes a list of pairs of register, value that generate the appropriate code.
    macro CRTC_SET, reg, val
        ; generate crtc reg selection only if needed
        if CRTC_SET_SELECTED_CRTC_REGISTER != {reg}
            if CRTC_SET_REG_C == '?' && CRTC_SET_REG_B == '?'
                ld bc, 0xbc00 + {reg}
            else 
                if CRTC_SET_REG_B == 0xbd 
                    dec b
                    if CRTC_SET_REG_C == {reg} - 1
                        inc c
                    else
                        ld c, {reg} ; XXX a better version will work with 16bits loads
                    endif
                else
                    print CRTC_SET_REG_B, " ", CRTC_SET_REG_C, " ", CRTC_SET_SELECTED_CRTC_REGISTER, " ", {reg}
                    fail "Unhandled case. Need to log more info to debug"
                endif
            endif
            CRTC_SET_REG_B = 0xbc
            CRTC_SET_REG_C = {reg}
            out (c), c
            CRTC_SET_SELECTED_CRTC_REGISTER = {reg}
        endif

        assert CRTC_SET_REG_B == 0xbc
        inc b
        CRTC_SET_REG_B = 0xbd

        ld a, {val}
        out (c), a
    endm


    ; Example of cuntion defintiion
    function load_palette
            palette = list_sublist(load("dummy_logo_palette.bin"), 0, 4) ; A sample of what can be done with basm functions
            print "Palette: ", \
                ga_to_name(list_get(palette,0)), "/", {hex}list_get(palette,0), " ", \
                ga_to_name(list_get(palette,1)), "/", {hex}list_get(palette,1), " ", \
                ga_to_name(list_get(palette,2)), "/", {hex}list_get(palette,2), " ", \
                ga_to_name(list_get(palette,3)), "/", {hex}list_get(palette,3)
            
            return palette
    endfunction
    palette = load_palette() ; Palette contains the list of 4 bytes

    assert $ == 0x4000, "No data should have been produced until here !"

init
    ; Kill the system
    di
        ld hl, 0xc9fb ; TODO use a function
        ld (0x38), hl ; TODO use a symbolic value
        ld sp, $
    ei
    
    call copy_logo_on_screen
    call setup_crtc_values
    call setup_ga_values
main


.setup_screen

    jp $

; Copy the logo at the right position in memory
copy_logo_on_screen

    ld hl, logo_data.start
    ld de, SCREEN_MEMORY_ADDRESS
    
    ld b, dummy_logo_conf_HEIGHT : assert dummy_logo_conf_HEIGHT < 256
.loop
        push bc
        push de
            ld bc, dummy_logo_conf_WIDTH
            BREAKPOINT
            ldir
        pop de
            ex de, hl
            call BC26
            ex de, hl
        pop BC
    djnz .loop

    ret

BC26 LD A,H      
     ADD A,8     
     LD H,A
     RET NC      
     LD BC, 0xC000 + 96 
     ADD HL,BC
     RET

setup_crtc_values
    CRTC_SET_INIT_STATE (void)
    CRTC_SET CRTC_REG_HORIZONTAL_DISPLAYED, dummy_logo_conf_WIDTH/2
    CRTC_SET CRTC_REG_HORIZONTAL_SYNC_POSITION, 50
    CRTC_SET CRTC_REG_VERTICAL_DISPLAYED, dummy_logo_conf_HEIGHT/8 +1
    ret 

;;
; Here we use some macros and functiosn to handle the generation of the code
; dedicated to palette choice
setup_ga_values
    macro GA_SET_INK_RESET_STATE_MACHINE pos, max
        GA_SET_INK_PEN_POSITION = {pos}
        GA_SET_INK_MAX_PEN_POSITION = {max}
        ld bc, 0x7F00 + GA_SET_INK_PEN_POSITION
    endm

    ; No-renentrante macro. A state is handle in GA_SET_INK_PEN_POSITION to read the appropriate
    macro GA_SET_INK
        out (c), c ; select the pen
        ld a, list_get(palette, GA_SET_INK_PEN_POSITION) ; read the palette value
        out (c), a ; set the appropriate ink

        if GA_SET_INK_MAX_PEN_POSITION != GA_SET_INK_PEN_POSITION
            inc c; go to next pen position in the assembled code ...
            GA_SET_INK_PEN_POSITION = GA_SET_INK_PEN_POSITION+1 ; ... and the state
        endif
    endm

    ; BUG: there is a bug in the palette order generation of img2cpc. This tool needs to be fixed
.generated_start
    ; Generate the appropriate code
    GA_SET_INK_RESET_STATE_MACHINE 0, 3
    repeat  4
        GA_SET_INK (void)
    endr
.generated_end

    ; print "Generated code\n", disassemble(.generated_start, .generated_end) TODO add such directive ;)
    ret

    if False
logo_data
.start
    incbin "dummy_logo.o"
.end
    ; Next assert fails : I must have a bug somewhere !!! incbin seems to have removed 128 bytes (i.e the header)
    assert logo_data.end - logo_data.start == dummy_logo_conf_WIDTH*dummy_logo_conf_HEIGHT
    else
    ; workaround while I have not fixed incbin bug...
logo_data
.start
    db load("dummy_logo.o")
.end
    assert logo_data.end - logo_data.start == dummy_logo_conf_WIDTH*dummy_logo_conf_HEIGHT
    endif

; Read only 4 bytes from the palette file
logo_palette
.start
    ; incbin "dummy_logo_palette.bin" ; <= standard way to include the complete file

    db palette
.end
    assert .end - .start == 4, "Only 4 bytes are needed in the palette"

    assert .end-.start < 0x4000, "Overscan expected here"
    assert $ < 0xc000
