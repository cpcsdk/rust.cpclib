ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #31 : ld h,a
;*** line 61/72 ***
ld bc,#55
ld de,#40AA
ld a,l : add 5 : ld l,a : ld (hl),#88
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 62/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),#80
res 4,h
;*** line 63/72 ***
ld (hl),#20
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 64/72 ***
inc l : ld (hl),b
inc l : ld (hl),b
inc l : inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld (hl),b
dec l : ld (hl),#1A
dec l : ld (hl),b
dec l : ld (hl),#F0
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 66/72 ***
ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),#85
inc l : ld (hl),#20
inc l : ld (hl),#1A
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
dec l : ld (hl),#8A
dec l : ld (hl),#25
dec l : ld (hl),#45
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#50
inc l : ld (hl),#F
inc l : ld (hl),#A
set 5,h
;*** line 69/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#4A
dec l : ld (hl),#D0
dec l : ld (hl),#80
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),d
inc l : ld (hl),#44
inc l : ld (hl),#E0
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 71/72 ***
ld (hl),#20
dec l : ld (hl),#1A
dec l : ld (hl),#50
dec l : ld (hl),#C0
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#45
inc l : ld (hl),#F
inc l : ld (hl),#A
ret
; #DBG curx=5 flux=83 on 720
