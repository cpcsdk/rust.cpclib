ex hl,de
;*** line 1/16 ***
ld a,h : add 48 : ld h,a
;*** line 5/16 ***
ld bc,#5
ld de,#AA20
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#50
set 3,h
;*** line 6/16 ***
ld (hl),b
inc l : ld (hl),#90
inc l : ld a,(hl) : and #55 : ld (hl),a
res 4,h
;*** line 7/16 ***
dec l : ld (hl),b
dec l : ld (hl),#14
res 3,h
;*** line 8/16 ***
ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
inc l : inc l : inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),c
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 10/16 ***
inc l : ld (hl),c
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),b
set 4,h
;*** line 11/16 ***
inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),e
dec l : ld (hl),c
res 3,h
;*** line 12/16 ***
ld (hl),c
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 13/16 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#C4
dec l : ld (hl),#92
dec l : ld (hl),#44
set 3,h
;*** line 14/16 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#C0
inc l : ld (hl),#C8
inc l : ld (hl),b
res 4,h
;*** line 15/16 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),#F
dec l : ld (hl),c
res 3,h
;*** line 16/16 ***
ld (hl),c
inc l : ld (hl),#1A
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
ret
; #DBG curx=6 flux=55 on 112
