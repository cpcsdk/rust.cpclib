ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #31 : ld h,a
;*** line 45/72 ***
ld bc,#AA
ld de,#5580
inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 46/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#44
inc l : ld (hl),b
inc l : ld (hl),e
res 4,h
;*** line 47/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 48/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 50/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#88
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 51/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),#40
dec l : ld (hl),b
res 3,h
;*** line 52/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 53/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 54/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),#E0
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 56/72 ***
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),#DA
dec l : ld (hl),#F
dec l : ld (hl),b
set 3,h
;*** line 58/72 ***
ld (hl),b
inc l : ld (hl),#30
inc l : ld (hl),#F
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 59/72 ***
dec l : ld (hl),#20
dec l : ld (hl),#A5
dec l : ld (hl),#1A
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 60/72 ***
ld (hl),b
inc l : ld (hl),#5
inc l : ld (hl),#A5
inc l : ld (hl),#D8
inc l : ld (hl),b
set 5,h
;*** line 61/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#E4
dec l : ld (hl),#30
dec l : ld (hl),#4A
dec l : ld (hl),#F
dec l : ld (hl),b
set 3,h
;*** line 62/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),b
inc l : ld (hl),#5E
inc l : ld (hl),#E4
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#E0
dec l : ld (hl),#E5
dec l : ld (hl),#F
dec l : ld (hl),#F
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#30
inc l : ld (hl),#25
inc l : ld (hl),#F
inc l : ld (hl),#8A
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#FC
dec l : ld (hl),#FC
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 66/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#AC
inc l : ld (hl),#FC
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 67/72 ***
dec l : ld (hl),#AC
dec l : ld (hl),#5
res 3,h
;*** line 68/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#AC
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 69/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#E8
dec l : ld (hl),#10
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 70/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#11
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 71/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AC
dec l : ld (hl),#5
res 3,h
;*** line 72/72 ***
ld (hl),#5
inc l : ld (hl),#AC
ret
; #DBG curx=6 flux=172 on 720
