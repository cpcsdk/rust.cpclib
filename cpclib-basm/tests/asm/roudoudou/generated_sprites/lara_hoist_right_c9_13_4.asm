ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#55
ld de,#AA40
inc l : inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/72 ***
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 61/72 ***
inc l : inc l : inc l : ld (hl),#20
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 62/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#1A
inc l : ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),#88
res 4,h
;*** line 63/72 ***
ld (hl),#A0
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
res 3,h
;*** line 64/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld (hl),#80
dec l : ld (hl),#4F
dec l : ld (hl),#A
dec l : ld (hl),#65
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),#64
inc l : ld (hl),#1A
inc l : ld (hl),#A
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
dec l : ld (hl),#A0
dec l : ld (hl),#F
dec l : ld (hl),#5
dec l : ld (hl),#C0
dec l : ld (hl),#44
dec l : ld (hl),b
res 3,h
;*** line 68/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),#85
inc l : ld (hl),#F
inc l : ld (hl),#A
set 5,h
;*** line 69/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C0
dec l : ld (hl),#50
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#A
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 71/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#5
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 72/72 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),#85
inc l : ld (hl),#F
inc l : ld (hl),#20
ret
; #DBG curx=5 flux=98 on 720
