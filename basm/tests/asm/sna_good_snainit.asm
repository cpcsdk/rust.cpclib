    SNAINIT "hello.sna"
    BANKSET 0

    MACRO PLOT x, y
        delta = {x}>>2

        SWITCH {x}&3
            CASE 0: pix=%10000000 : BREAK
            CASE 1: pix=%01000000 : BREAK
            CASE 2: pix=%00100000 : BREAK
            CASE 3: pix=%00010000 : BREAK
        ENDSWITCH
            
        addy = ({y}&7)*#800 + ({y}>>3)*80
        adr = #C000 + delta + addy

        print "Plot in ", {hex}adr, " for ", {x}, ",", {y}
        ld hl, adr : ld a, pix : or (hl) : ld (hl),a
    
         
    ENDM


    org #8000
    radius = 80
    repeat 360, x
        PLOT {eval}int(160+radius*cos({x})), {eval}int(100+radius*sin({x}))
    rend
    ret 