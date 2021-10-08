ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #31 : ld h,a
;*** line 45/72 ***
ld bc,#AA
ld de,#5540
ld a,l : add 5 : ld l,a : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 46/72 ***
dec l : dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #C8 : ld h,a
;*** line 49/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 50/72 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#44
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 51/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 52/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#88
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 53/72 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 54/72 ***
ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#50
inc l : ld (hl),b
res 4,h
;*** line 55/72 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 56/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#80
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld (hl),b
dec l : ld (hl),#E4
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
set 3,h
;*** line 58/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#D0
inc l : ld (hl),#4E
inc l : ld (hl),b
set 4,h
;*** line 59/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),#1A
dec l : ld (hl),#41
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 60/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#94
inc l : ld (hl),#F
inc l : ld (hl),#A
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 61/72 ***
ld (hl),#80
dec l : ld (hl),#87
dec l : ld (hl),#94
dec l : ld (hl),#F
dec l : ld (hl),#5
set 3,h
;*** line 62/72 ***
ld (hl),#50
inc l : ld (hl),#D2
inc l : ld (hl),e
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#1E
dec l : ld (hl),#94
dec l : ld (hl),#F
dec l : ld (hl),#10
res 3,h
;*** line 64/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#60
inc l : ld (hl),#98
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#2D
dec l : ld (hl),#50
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#50
inc l : ld (hl),#AD
inc l : ld a,(hl) : and d : or #20 :ld (hl),a
set 4,h
;*** line 67/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#FC
dec l : ld (hl),#5C
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5C
inc l : ld (hl),#FC
inc l : ld a,(hl) : and d : or #20 :ld (hl),a
set 5,h
;*** line 69/72 ***
dec l : ld (hl),#A
dec l : ld (hl),#54
set 3,h
;*** line 70/72 ***
ld (hl),#54
inc l : ld (hl),#1A
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 71/72 ***
dec l : ld (hl),#A
dec l : ld (hl),#54
res 3,h
;*** line 72/72 ***
ld (hl),#5C
inc l : ld (hl),#A8
inc l : ld a,(hl) : and d : ld (hl),a
ret
; #DBG curx=5 flux=150 on 792
