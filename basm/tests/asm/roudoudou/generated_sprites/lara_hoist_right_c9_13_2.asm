ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#55
ld de,#AAA
ld a,l : add 5 : ld l,a : ld a,(hl) : and c : ld (hl),a
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
set 5,h
;*** line 61/72 ***
dec l : dec l : dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),#A0
set 3,h
;*** line 62/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#30
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 63/72 ***
inc l : inc l : inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#85
inc l : ld (hl),#88
res 3,h
;*** line 64/72 ***
ld (hl),#80
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : dec l : dec l : dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 66/72 ***
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),#E0
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 67/72 ***
ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#85
inc l : ld (hl),#30
inc l : ld (hl),d
res 3,h
;*** line 68/72 ***
ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),#85
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 69/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#D0
inc l : ld (hl),#F
inc l : ld (hl),#20
set 3,h
;*** line 70/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#4A
dec l : ld (hl),#C4
dec l : ld (hl),#C0
dec l : ld (hl),#40
dec l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 71/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#D0
inc l : ld (hl),#A5
inc l : ld (hl),#88
res 3,h
;*** line 72/72 ***
ld (hl),#8A
dec l : ld (hl),#8D
dec l : ld (hl),#D0
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
ret
; #DBG curx=0 flux=76 on 720
