ex hl,de
;*** line 1/72 ***
ld a,l : add 128 : ld l,a : ld a,h : adc #9 : ld h,a
;*** line 50/72 ***
ld bc,#AA
ld de,#5540
inc l : inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 51/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/72 ***
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 53/72 ***
ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#C4
inc l : ld (hl),e
inc l : ld (hl),b
set 3,h
;*** line 54/72 ***
ld (hl),b
dec l : ld (hl),#D0
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/72 ***
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 56/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#80
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),#D0
inc l : ld (hl),b
set 3,h
;*** line 58/72 ***
ld (hl),b
dec l : ld (hl),#A5
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 59/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#30
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/72 ***
ld (hl),b
dec l : ld (hl),#A5
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
set 5,h
;*** line 61/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#A5
inc l : ld (hl),#1A
inc l : ld (hl),#5A
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 62/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CE
dec l : ld (hl),#85
dec l : ld (hl),#90
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#8D
inc l : ld (hl),#F
inc l : ld (hl),#E0
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 64/72 ***
dec l : ld (hl),b
dec l : ld (hl),#1A
dec l : ld (hl),#45
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),#90
inc l : ld (hl),#E5
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#80
dec l : ld (hl),#F
dec l : ld (hl),#50
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
inc l : ld (hl),b
inc l : ld (hl),#45
inc l : ld (hl),#F
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 68/72 ***
ld (hl),b
dec l : ld (hl),#DA
dec l : ld (hl),#5
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 69/72 ***
inc l : inc l : ld (hl),#10
inc l : ld (hl),#FC
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AC
dec l : ld (hl),#54
res 4,h
;*** line 71/72 ***
ld (hl),#D0
inc l : ld (hl),#FC
inc l : ld (hl),#20
res 3,h
;*** line 72/72 ***
ld a,(hl) : and d : or #20 :ld (hl),a
dec l : ld (hl),#A5
dec l : ld (hl),#50
dec l : ld a,(hl) : and c : ld (hl),a
ret
; #DBG curx=2 flux=145 on 720
