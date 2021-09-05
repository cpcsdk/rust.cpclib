; Test of assembling of z80 instructions.
    LIST

    org $0

des equ $05
n   equ $20
nn  equ $0584

    ; Documented instructions
;   ADC
    adc a,(hl)
    adc a,(ix + des)
    adc a,(iy + des)
    adc a,a
    adc a,b
    adc a,c
    adc a,d
    adc a,e
    adc a,h
    adc a,l
    adc a,n

    adc hl,bc
    adc hl,de
    adc hl,hl
    adc hl,sp

;   ADD
    add a,(hl)
    add a,(ix + des)
    add a,(iy + des)
    add a,a
    add a,b
    add a,c
    add a,d
    add a,e
    add a,h
    add a,l
    add a,n

    add hl,bc
    add hl,de
    add hl,hl
    add hl,sp

    add ix,bc
    add ix,de
    add ix,ix
    add ix,sp

    add iy,bc
    add iy,de
    add iy,iy
    add iy,sp

;   AND
    and (hl)
    and (ix + des)
    and (iy + des)
    and a
    and b
    and c
    and d
    and e
    and h
    and l
    and n

;   BIT
    bit 0,(hl)
    bit 0,(ix + des)
    bit 0,(iy + des)
    bit 0,a
    bit 0,b
    bit 0,c
    bit 0,d
    bit 0,e
    bit 0,h
    bit 0,l

    bit 1,(hl)
    bit 1,(ix + des)
    bit 1,(iy + des)
    bit 1,a
    bit 1,b
    bit 1,c
    bit 1,d
    bit 1,e
    bit 1,h
    bit 1,l

    bit 2,(hl)
    bit 2,(ix + des)
    bit 2,(iy + des)
    bit 2,a
    bit 2,b
    bit 2,c
    bit 2,d
    bit 2,e
    bit 2,h
    bit 2,l

    bit 3,(hl)
    bit 3,(ix + des)
    bit 3,(iy + des)
    bit 3,a
    bit 3,b
    bit 3,c
    bit 3,d
    bit 3,e
    bit 3,h
    bit 3,l

    bit 4,(hl)
    bit 4,(ix + des)
    bit 4,(iy + des)
    bit 4,a
    bit 4,b
    bit 4,c
    bit 4,d
    bit 4,e
    bit 4,h
    bit 4,l

    bit 5,(hl)
    bit 5,(ix + des)
    bit 5,(iy + des)
    bit 5,a
    bit 5,b
    bit 5,c
    bit 5,d
    bit 5,e
    bit 5,h
    bit 5,l

    bit 6,(hl)
    bit 6,(ix + des)
    bit 6,(iy + des)
    bit 6,a
    bit 6,b
    bit 6,c
    bit 6,d
    bit 6,e
    bit 6,h
    bit 6,l

    bit 7,(hl)
    bit 7,(ix + des)
    bit 7,(iy + des)
    bit 7,a
    bit 7,b
    bit 7,c
    bit 7,d
    bit 7,e
    bit 7,h
    bit 7,l

;   CALL
    call nn

    call nz,nn
    call z,nn
    call nc,nn
    call c,nn
    call po,nn
    call pe,nn
    call p,nn
    call m,nn

;   CCF
    ccf

;   CP
    cp (hl)
    cp (ix + des)
    cp (iy + des)
    cp a
    cp b
    cp c
    cp d
    cp e
    cp h
    cp l
    cp n

    cpd
    cpdr
    cpir
    cpi

;   CPL
    cpl

;   DAA
    daa

;   DEC
    dec (hl)
    dec (ix + des)
    dec (iy + des)
    dec a
    dec b
    dec c
    dec d
    dec e
    dec h
    dec l

    dec bc
    dec de
    dec hl
    dec ix
    dec iy
    dec sp

;   DI
    di

;   DJNZ
l1  djnz l1

;   EI
    ei

;   EX
    ex af,af'

    ex de,hl

    ex (sp),hl
    ex (sp),ix
    ex (sp),iy

    exx

;   HALT
    halt

;   IM
    im 0
    im 1
    im 2

;   IN
    in a,(c)
    in b,(c)
    in c,(c)
    in d,(c)
    in e,(c)
    in h,(c)
    in l,(c)

    in a,(n)

    ind
    indr
    ini
    inir

;   INC
    inc (hl)
    inc (ix + des)
    inc (iy + des)
    inc a
    inc b
    inc c
    inc d
    inc e
    inc h
    inc l

    inc bc
    inc de
    inc hl
    inc ix
    inc iy
    inc sp

;   JP
    jp nn

    jp (hl)
    jp (ix)
    jp (iy)

    jp nz,nn
    jp z,nn
    jp nc,nn
    jp c,nn
    jp po,nn
    jp pe,nn
    jp p,nn
    jp m,nn

;   JR
    jr $ + $22

    jr nz,$ + $22
    jr z,$ + $22
    jr nc,$ + $22
    jr c,$ + $22

;   LD
    ld (bc),a
    ld (de),a

    ld (hl),a
    ld (hl),b
    ld (hl),c
    ld (hl),d
    ld (hl),e
    ld (hl),h
    ld (hl),l
    ld (hl),n

    ld (ix + des),a
    ld (ix + des),b
    ld (ix + des),c
    ld (ix + des),d
    ld (ix + des),e
    ld (ix + des),h
    ld (ix + des),l
    ld (ix + des),n

    ld (iy + des),a
    ld (iy + des),b
    ld (iy + des),c
    ld (iy + des),d
    ld (iy + des),e
    ld (iy + des),h
    ld (iy + des),l
    ld (iy + des),n

    ld (nn),a

    ld (nn),bc
    ld (nn),de
    ld (nn),hl
    ld (nn),ix
    ld (nn),iy

    ld (nn),sp

    ld a,(bc)
    ld a,(de)
    ld a,(hl)
    ld a,(ix + des)
    ld a,(iy + des)
    ld a,(nn)
    ld a,a
    ld a,b
    ld a,c
    ld a,d
    ld a,e
    ld a,h
    ld a,l
    ld a,n

    ld b,(hl)
    ld b,(ix + des)
    ld b,(iy + des)
    ld b,a
    ld b,b
    ld b,c
    ld b,d
    ld b,e
    ld b,h
    ld b,l
    ld b,n

    ld c,(hl)
    ld c,(ix + des)
    ld c,(iy + des)
    ld c,a
    ld c,b
    ld c,c
    ld c,d
    ld c,e
    ld c,h
    ld c,l
    ld c,n

    ld d,(hl)
    ld d,(ix + des)
    ld d,(iy + des)
    ld d,a
    ld d,b
    ld d,c
    ld d,d
    ld d,e
    ld d,h
    ld d,l
    ld d,n

    ld e,(hl)
    ld e,(ix + des)
    ld e,(iy + des)
    ld e,a
    ld e,b
    ld e,c
    ld e,d
    ld e,e
    ld e,h
    ld e,l
    ld e,n

    ld h,(hl)
    ld h,(ix + des)
    ld h,(iy + des)
    ld h,a
    ld h,b
    ld h,c
    ld h,d
    ld h,e
    ld h,h
    ld h,l
    ld h,n

    ld l,(hl)
    ld l,(ix + des)
    ld l,(iy + des)
    ld l,a
    ld l,b
    ld l,c
    ld l,d
    ld l,e
    ld l,h
    ld l,l
    ld l,n

    ld a,i
    ld i,a

    ld a,r
    ld r,a
    
    ld bc,(nn)
    ld de,(nn)
    ld hl,(nn)
    ld ix,(nn)
    ld iy,(nn)
    ld sp,(nn)

    ld bc,nn
    ld de,nn
    ld hl,nn
    ld ix,nn
    ld iy,nn


    ld sp,hl
    ld sp,ix
    ld sp,iy
    ld sp,nn

    ldd
    lddr
    ldi
    ldir

;   NEG
    neg

;   NOP
    nop

;   OR
    or (hl)
    or (ix + des)
    or (iy + des)
    or a
    or b
    or c
    or d
    or e
    or h
    or l
    or n


;   OUT
    out (c),a
    out (c),b
    out (c),c
    out (c),d
    out (c),e
    out (c),h
    out (c),l
    out (n),a

    outd
    otdr
    outi
    otir

;   POP
    pop af
    pop bc
    pop de
    pop hl
    pop ix
    pop iy

;   PUSH
    push af
    push bc
    push de
    push hl
    push ix
    push iy

;   RES
    res 0,(hl)
    res 0,(ix + des)
    res 0,(iy + des)
    res 0,a
    res 0,b
    res 0,c
    res 0,d
    res 0,e
    res 0,h
    res 0,l

    res 1,(hl)
    res 1,(ix + des)
    res 1,(iy + des)
    res 1,a
    res 1,b
    res 1,c
    res 1,d
    res 1,e
    res 1,h
    res 1,l

    res 2,(hl)
    res 2,(ix + des)
    res 2,(iy + des)
    res 2,a
    res 2,b
    res 2,c
    res 2,d
    res 2,e
    res 2,h
    res 2,l

    res 3,(hl)
    res 3,(ix + des)
    res 3,(iy + des)
    res 3,a
    res 3,b
    res 3,c
    res 3,d
    res 3,e
    res 3,h
    res 3,l

    res 4,(hl)
    res 4,(ix + des)
    res 4,(iy + des)
    res 4,a
    res 4,b
    res 4,c
    res 4,d
    res 4,e
    res 4,h
    res 4,l

    res 5,(hl)
    res 5,(ix + des)
    res 5,(iy + des)
    res 5,a
    res 5,b
    res 5,c
    res 5,d
    res 5,e
    res 5,h
    res 5,l

    res 6,(hl)
    res 6,(ix + des)
    res 6,(iy + des)
    res 6,a
    res 6,b
    res 6,c
    res 6,d
    res 6,e
    res 6,h
    res 6,l

    res 7,(hl)
    res 7,(ix + des)
    res 7,(iy + des)
    res 7,a
    res 7,b
    res 7,c
    res 7,d
    res 7,e
    res 7,h
    res 7,l

;   RET
    ret

    ret z
    ret nz
    ret c
    ret nc
    ret po
    ret pe
    ret p
    ret m

    reti
    retn

;   RL
    rl (hl)
    rl (ix + des)
    rl (iy + des)
    rl a
    rl b
    rl c
    rl d
    rl e
    rl h
    rl l

;   RLA
    rla

;   RLC
    rlc (hl)
    rlc (ix + des)
    rlc (iy + des)
    rlc a
    rlc b
    rlc c
    rlc d
    rlc e
    rlc h
    rlc l

;   RLCA
    rlca

;   RLD
    rld

;   RR
    rr (hl)
    rr (ix + des)
    rr (iy + des)
    rr a
    rr b
    rr c
    rr d
    rr e
    rr h
    rr l

;   RRA
    rra

;   RRC
    rrc (hl)
    rrc (ix + des)
    rrc (iy + des)
    rrc a
    rrc b
    rrc c
    rrc d
    rrc e
    rrc h
    rrc l

;   RRCA
    rrca

;   RRD
    rrd

;   RST
    rst $00
    rst $08
    rst $10
    rst $18
    rst $20
    rst $28
    rst $30
    rst $38

;   SBC
    sbc a,(hl)
    sbc a,(ix + des)
    sbc a,(iy + des)
    sbc a,a
    sbc a,b
    sbc a,c
    sbc a,d
    sbc a,e
    sbc a,h
    sbc a,l
    sbc a,n

    sbc hl,bc
    sbc hl,de
    sbc hl,hl
    sbc hl,sp

;   SCF
    scf

;   SET
    set 0,(hl)
    set 0,(ix + des)
    set 0,(iy + des)
    set 0,a
    set 0,b
    set 0,c
    set 0,d
    set 0,e
    set 0,h
    set 0,l

    set 1,(hl)
    set 1,(ix + des)
    set 1,(iy + des)
    set 1,a
    set 1,b
    set 1,c
    set 1,d
    set 1,e
    set 1,h
    set 1,l

    set 2,(hl)
    set 2,(ix + des)
    set 2,(iy + des)
    set 2,a
    set 2,b
    set 2,c
    set 2,d
    set 2,e
    set 2,h
    set 2,l

    set 3,(hl)
    set 3,(ix + des)
    set 3,(iy + des)
    set 3,a
    set 3,b
    set 3,c
    set 3,d
    set 3,e
    set 3,h
    set 3,l

    set 4,(hl)
    set 4,(ix + des)
    set 4,(iy + des)
    set 4,a
    set 4,b
    set 4,c
    set 4,d
    set 4,e
    set 4,h
    set 4,l

    set 5,(hl)
    set 5,(ix + des)
    set 5,(iy + des)
    set 5,a
    set 5,b
    set 5,c
    set 5,d
    set 5,e
    set 5,h
    set 5,l

    set 6,(hl)
    set 6,(ix + des)
    set 6,(iy + des)
    set 6,a
    set 6,b
    set 6,c
    set 6,d
    set 6,e
    set 6,h
    set 6,l

    set 7,(hl)
    set 7,(ix + des)
    set 7,(iy + des)
    set 7,a
    set 7,b
    set 7,c
    set 7,d
    set 7,e
    set 7,h
    set 7,l

;   SLA
    sla (hl)
    sla (ix + des)
    sla (iy + des)
    sla a
    sla b
    sla c
    sla d
    sla e
    sla h
    sla l

;   SRA
    sra (hl)
    sra (ix + des)
    sra (iy + des)
    sra a
    sra b
    sra c
    sra d
    sra e
    sra h
    sra l

;   SRL
    srl (hl)
    srl (ix + des)
    srl (iy + des)
    srl a
    srl b
    srl c
    srl d
    srl e
    srl h
    srl l

;   SUB
    sub (hl)
    sub (ix + des)
    sub (iy + des)
    sub a
    sub b
    sub c
    sub d
    sub e
    sub h
    sub l
    sub n

;   XOR
    xor (hl)
    xor (ix + des)
    xor (iy + des)
    xor a
    xor b
    xor c
    xor d
    xor e
    xor h
    xor l
    xor n

    ; Undocumented instructions
; IN
    in (c)      ; DEFB $ED,$70
    in f,(c)    ; DEFB $ED,$70

; OUT
    out (c)     ; DEFB $ED,$71
    out (c),f   ; DEFB $ED,$71

; SLL
    sll (hl)
    sll (ix+des)
    sll (iy+des)
    sll a
    sll b
    sll c
    sll d
    sll e
    sll h
    sll l

; IX and IY 8 bits halfs
    add a,ixh
    add a,ixl
    add a,iyh
    add a,iyl

    adc a,ixh
    adc a,ixl
    adc a,iyh
    adc a,iyl

    and ixh
    and ixl
    and iyh
    and iyl

    cp ixh
    cp ixl
    cp iyh
    cp iyl

    dec ixh
    dec ixl
    dec iyh
    dec iyl

    inc ixh
    inc ixl
    inc iyh
    inc iyl

    ld a,ixh
    ld b,ixh
    ld c,ixh
    ld d,ixh
    ld e,ixh
    ld h,ixh
    ld l,ixh

    ld a,ixl
    ld b,ixl
    ld c,ixl
    ld d,ixl
    ld e,ixl
    ld h,ixl
    ld l,ixl

    ld a,iyh
    ld b,iyh
    ld c,iyh
    ld d,iyh
    ld e,iyh
    ld h,iyh
    ld l,iyh

    ld a,iyl
    ld b,iyl
    ld c,iyl
    ld d,iyl
    ld e,iyl
    ld h,iyl
    ld l,iyl

    ld ixh,a
    ld ixh,b
    ld ixh,c
    ld ixh,d
    ld ixh,e
    ld ixh,ixh
    ld ixh,ixl
    ld ixh,n

    ld ixl,a
    ld ixl,b
    ld ixl,c
    ld ixl,d
    ld ixl,e
    ld ixl,ixh
    ld ixl,ixl
    ld ixl,n

    ld iyh,a
    ld iyh,b
    ld iyh,c
    ld iyh,d
    ld iyh,e
    ld iyh,iyh
    ld iyh,iyl
    ld iyh,n

    ld iyl,a
    ld iyl,b
    ld iyl,c
    ld iyl,d
    ld iyl,e
    ld iyl,iyh
    ld iyl,iyl
    ld iyl,n

    or ixh
    or ixl
    or iyh
    or iyl

    sbc a,ixh
    sbc a,ixl
    sbc a,iyh
    sbc a,iyl

    sub ixh
    sub ixl
    sub iyh
    sub iyl

    xor ixh
    xor ixl
    xor iyh
    xor iyl
 

    end
