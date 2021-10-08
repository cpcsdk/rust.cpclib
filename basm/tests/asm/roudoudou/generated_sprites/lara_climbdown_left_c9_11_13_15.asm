ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #9 : ld h,a
;*** line 42/72 ***
ld bc,#55
ld de,#AA80
ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 43/72 ***
dec l : dec l : dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 44/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 45/72 ***
dec l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 46/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 47/72 ***
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 48/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#44
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 50/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 51/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#C8
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 53/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#50
inc l : ld (hl),#69
inc l : ld (hl),#87
inc l : ld (hl),#A5
inc l : ld (hl),b
set 3,h
;*** line 54/72 ***
ld (hl),#A0
dec l : ld (hl),#2D
dec l : ld (hl),#F
dec l : ld (hl),#1A
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#D0
inc l : ld (hl),e
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 56/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#1E
inc l : ld (hl),#F
inc l : ld (hl),#F
inc l : ld (hl),#A0
set 3,h
;*** line 58/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#5A
dec l : ld (hl),#1A
dec l : ld (hl),#5
dec l : ld (hl),b
set 4,h
;*** line 59/72 ***
inc l : ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),#56
inc l : ld (hl),#FC
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 60/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A4
dec l : ld (hl),#92
dec l : ld (hl),#F
dec l : ld (hl),#10
dec l : ld (hl),b
set 5,h
;*** line 61/72 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#C8
inc l : ld (hl),#44
inc l : ld (hl),#24
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 62/72 ***
ld (hl),b
dec l : ld (hl),#20
dec l : ld (hl),#90
dec l : ld (hl),#CC
dec l : ld (hl),#C0
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#B4
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),#24
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#AC
dec l : ld (hl),#FC
dec l : ld (hl),#6C
dec l : ld (hl),#5
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
inc l : ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#90
inc l : ld (hl),#20
inc l : ld (hl),b
set 3,h
;*** line 66/72 ***
ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),#C8
dec l : ld (hl),#44
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 67/72 ***
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),b
inc l : ld (hl),#88
res 3,h
;*** line 68/72 ***
ld (hl),#88
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),#44
set 5,h
;*** line 69/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#40
inc l : ld (hl),b
set 3,h
;*** line 70/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),e
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 71/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),#44
inc l : ld (hl),e
res 3,h
;*** line 72/72 ***
ld (hl),#88
dec l : ld (hl),#40
dec l : ld (hl),#C8
dec l : ld a,(hl) : and d : ld (hl),a
ret
; #DBG curx=5 flux=199 on 792
