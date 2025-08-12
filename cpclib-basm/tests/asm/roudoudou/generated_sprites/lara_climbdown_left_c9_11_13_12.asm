ex hl,de
;*** line 1/72 ***
ld a,h : add 49 : ld h,a
;*** line 37/72 ***
ld bc,#AA
ld de,#5540
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 38/72 ***
ld (hl),e
inc l : ld (hl),#C0
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 39/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 41/72 ***
dec l : dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 42/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 43/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 44/72 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 45/72 ***
ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 46/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),#44
dec l : ld (hl),b
res 4,h
;*** line 47/72 ***
ld (hl),b
inc l : ld (hl),#44
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 48/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),#50
inc l : ld (hl),#5A
inc l : ld (hl),b
set 3,h
;*** line 50/72 ***
ld (hl),#A0
dec l : ld (hl),#5A
dec l : ld (hl),#C1
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
set 4,h
;*** line 51/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#F
inc l : ld (hl),#5A
inc l : ld (hl),#B1
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#D2
dec l : ld (hl),#1E
dec l : ld (hl),e
dec l : ld (hl),#44
dec l : ld (hl),b
set 5,h
;*** line 53/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#98
inc l : ld (hl),#F
inc l : ld (hl),#5E
inc l : ld (hl),#FC
inc l : ld (hl),#8
set 3,h
;*** line 54/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),#5E
dec l : ld (hl),#A
dec l : ld (hl),#90
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#50
inc l : ld (hl),#1A
inc l : ld (hl),#F
inc l : ld (hl),#5E
inc l : ld (hl),#FC
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 56/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#FC
dec l : ld (hl),#E
dec l : ld (hl),#F
dec l : ld (hl),#B4
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#44
inc l : ld (hl),#A
inc l : ld (hl),#30
inc l : ld (hl),#20
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 58/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#4E
dec l : ld (hl),#88
dec l : ld (hl),#44
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 59/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#C0
inc l : ld (hl),#C4
inc l : ld (hl),#44
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#44
dec l : ld (hl),#64
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 61/72 ***
dec l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#CC
inc l : ld (hl),#88
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 62/72 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#CC
inc l : ld (hl),#88
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#80
dec l : ld (hl),#CC
dec l : ld (hl),#C6
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#80
set 3,h
;*** line 66/72 ***
ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C0
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#80
inc l : inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 69/72 ***
inc l : ld (hl),b
inc l : ld (hl),#AC
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AC
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 71/72 ***
ld (hl),e
inc l : ld (hl),#84
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 72/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
ret
; #DBG curx=4 flux=237 on 792
