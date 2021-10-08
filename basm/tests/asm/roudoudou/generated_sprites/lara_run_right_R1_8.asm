ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #30 : ld h,a
;*** line 13/64 ***
ld bc,#55
ld de,#40AA
inc l : inc l : inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 14/64 ***
ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 15/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 16/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 18/64 ***
ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 19/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 20/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,h : add 16 : ld h,a
;*** line 24/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #F8 : ld h,a
;*** line 27/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
set 5,h
;*** line 29/64 ***
ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 30/64 ***
ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 31/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 32/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 34/64 ***
ld (hl),b
set 4,h
;*** line 35/64 ***
ld (hl),d
res 3,h
;*** line 36/64 ***
ld (hl),b
set 5,h
;*** line 37/64 ***
ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 38/64 ***
ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 39/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 40/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 42/64 ***
ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 43/64 ***
ld (hl),#44
res 3,h
;*** line 44/64 ***
ld (hl),b
set 5,h
;*** line 45/64 ***
ld (hl),#CC
dec l : ld (hl),#C4
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 46/64 ***
ld (hl),#4
inc l : ld (hl),#64
inc l : ld (hl),#CC
res 4,h
;*** line 47/64 ***
ld (hl),#CC
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 48/64 ***
inc l : ld (hl),b
inc l : ld (hl),#C4
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld (hl),#88
dec l : ld (hl),#4E
dec l : ld (hl),#10
set 3,h
;*** line 50/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),#A
inc l : ld (hl),b
set 4,h
;*** line 51/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),b
res 3,h
;*** line 52/64 ***
ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),b
set 5,h
;*** line 53/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
set 3,h
;*** line 54/64 ***
ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
res 3,h
;*** line 56/64 ***
ld (hl),d
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
set 3,h
;*** line 58/64 ***
ld a,(hl) : and e : ld (hl),a
;*** line 59/64 ***
;*** line 60/64 ***
;*** line 61/64 ***
;*** line 62/64 ***
;*** line 63/64 ***
;*** line 64/64 ***
ret
; #DBG curx=7 flux=91 on 1600
