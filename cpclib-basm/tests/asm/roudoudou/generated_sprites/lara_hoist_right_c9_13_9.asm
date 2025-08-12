ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #31 : ld h,a
;*** line 45/72 ***
ld bc,#AA
ld de,#5540
ld a,l : add 6 : ld l,a : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),e
set 3,h
;*** line 46/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#80
inc l : ld (hl),b
res 4,h
;*** line 47/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld (hl),#80
dec l : ld (hl),b
res 3,h
;*** line 48/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 50/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld (hl),b
set 4,h
;*** line 51/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 52/72 ***
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#80
inc l : ld (hl),b
set 5,h
;*** line 53/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 54/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#4F
inc l : ld (hl),#8F
inc l : ld (hl),#80
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 56/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#25
dec l : ld (hl),#F
dec l : ld (hl),#70
dec l : ld (hl),b
set 3,h
;*** line 58/72 ***
ld (hl),#50
inc l : ld (hl),#5A
inc l : ld (hl),#F
inc l : ld (hl),#20
inc l : ld (hl),b
set 4,h
;*** line 59/72 ***
dec l : ld (hl),#20
dec l : ld (hl),#90
dec l : ld (hl),#CF
dec l : ld (hl),#D0
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 60/72 ***
inc l : ld (hl),#50
inc l : ld (hl),#F
inc l : ld (hl),#25
inc l : ld (hl),#20
inc l : ld (hl),b
set 5,h
;*** line 61/72 ***
ld (hl),#80
dec l : ld (hl),#F
dec l : ld (hl),#25
dec l : ld (hl),#FC
dec l : ld (hl),#FC
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 62/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5C
inc l : ld (hl),#FC
inc l : ld (hl),e
inc l : ld (hl),#10
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#1A
dec l : ld (hl),#85
dec l : ld (hl),#AD
dec l : ld (hl),#72
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#D0
inc l : ld (hl),#70
inc l : ld (hl),#90
inc l : ld (hl),#A
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#20
dec l : ld (hl),#5C
set 3,h
;*** line 66/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#8
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 67/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#44
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 69/72 ***
dec l : ld (hl),#88
dec l : ld (hl),#C4
dec l : ld (hl),b
set 3,h
;*** line 70/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#64
inc l : ld (hl),b
res 4,h
;*** line 71/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#44
dec l : ld (hl),b
res 3,h
;*** line 72/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
ret
; #DBG curx=5 flux=179 on 720
