ex hl,de
;*** line 1/72 ***
ld a,l : add 128 : ld l,a : ld a,h : adc #31 : ld h,a
;*** line 53/72 ***
ld bc,#AA
ld de,#5540
inc l : inc l : inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 54/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 57/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 58/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 59/72 ***
ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),#C8
inc l : ld (hl),#B0
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 61/72 ***
ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),#9A
inc l : ld (hl),#B0
inc l : ld (hl),#A
set 3,h
;*** line 62/72 ***
ld (hl),#20
dec l : ld (hl),#A5
dec l : ld (hl),#F
dec l : ld (hl),#C0
dec l : ld (hl),#80
dec l : ld (hl),b
res 4,h
;*** line 63/72 ***
ld (hl),e
inc l : ld (hl),#C8
inc l : ld (hl),#80
inc l : ld (hl),#F0
inc l : ld (hl),#10
inc l : ld (hl),#A0
res 3,h
;*** line 64/72 ***
ld (hl),#88
dec l : ld (hl),b
dec l : ld (hl),#B0
dec l : ld (hl),#C0
dec l : ld (hl),#C4
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#F
inc l : ld (hl),#25
inc l : ld (hl),#C8
set 3,h
;*** line 66/72 ***
ld (hl),#80
dec l : ld (hl),#4F
dec l : ld (hl),#5A
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),b
res 3,h
;*** line 68/72 ***
inc l : ld (hl),b
dec l : ld (hl),#30
dec l : ld (hl),#CA
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 69/72 ***
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#9A
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 70/72 ***
ld (hl),#20
dec l : ld (hl),#A5
dec l : ld (hl),#5
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 71/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#5
inc l : ld (hl),#4A
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#5A
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
ret
; #DBG curx=0 flux=125 on 720
