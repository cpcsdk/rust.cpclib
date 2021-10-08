ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#AA
ld de,#555
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
set 5,h
;*** line 61/72 ***
ld (hl),#40
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 62/72 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/72 ***
ld (hl),#44
inc l : ld (hl),#80
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 64/72 ***
dec l : ld (hl),b
dec l : ld (hl),#40
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#D0
dec l : ld (hl),b
dec l : ld (hl),d
set 4,h
;*** line 67/72 ***
ld (hl),d
inc l : ld (hl),#30
inc l : ld (hl),#4A
inc l : ld (hl),#80
inc l : ld (hl),b
res 3,h
;*** line 68/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#4A
dec l : ld (hl),b
dec l : ld (hl),d
set 5,h
;*** line 69/72 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#E0
inc l : ld (hl),#C4
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),#C8
dec l : ld (hl),#F
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 71/72 ***
ld (hl),#44
inc l : ld (hl),#F
inc l : ld (hl),#68
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),#28
dec l : ld (hl),#1A
dec l : ld (hl),d
ret
; #DBG curx=5 flux=76 on 792
