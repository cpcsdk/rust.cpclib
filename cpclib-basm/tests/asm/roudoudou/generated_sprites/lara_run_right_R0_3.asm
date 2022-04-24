ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #30 : ld h,a
;*** line 13/64 ***
ld bc,#55
ld de,#40AA
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 14/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#C0
res 4,h
;*** line 15/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 17/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
set 3,h
;*** line 18/64 ***
ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 19/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 20/64 ***
ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 21/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
set 3,h
;*** line 22/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 23/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 24/64 ***
ld (hl),b
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #F8 : ld h,a
;*** line 27/64 ***
inc l : inc l : ld (hl),b
res 3,h
;*** line 28/64 ***
ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 29/64 ***
dec l : ld a,(hl) : and e : or d :ld (hl),a
inc l : ld (hl),#F
set 3,h
;*** line 30/64 ***
ld (hl),#1A
dec l : ld (hl),#50
res 4,h
;*** line 31/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#A5
res 3,h
;*** line 32/64 ***
ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
ld (hl),#5
inc l : ld (hl),#A
set 3,h
;*** line 34/64 ***
ld (hl),#20
dec l : ld (hl),#5
dec l : ld (hl),b
set 4,h
;*** line 35/64 ***
ld (hl),#5
inc l : ld (hl),#1A
inc l : ld (hl),b
res 3,h
;*** line 36/64 ***
ld (hl),b
dec l : ld (hl),#F
dec l : ld (hl),#50
set 5,h
;*** line 37/64 ***
ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#44
set 3,h
;*** line 38/64 ***
ld (hl),#44
res 4,h
;*** line 39/64 ***
dec l : dec l : ld (hl),#44
inc l : ld (hl),#20
inc l : ld (hl),b
res 3,h
;*** line 40/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),#C5
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
inc l : inc l : ld (hl),d
set 3,h
;*** line 42/64 ***
ld (hl),d
set 4,h
;*** line 43/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 44/64 ***
ld (hl),b
set 5,h
;*** line 45/64 ***
ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 46/64 ***
ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 47/64 ***
ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 48/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld (hl),b
set 3,h
;*** line 50/64 ***
ld (hl),d
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 51/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
res 3,h
;*** line 52/64 ***
ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 53/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 54/64 ***
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 55/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 56/64 ***
ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),b
;*** line 57/64 ***
;*** line 58/64 ***
;*** line 59/64 ***
;*** line 60/64 ***
;*** line 61/64 ***
;*** line 62/64 ***
;*** line 63/64 ***
;*** line 64/64 ***
ret
; #DBG curx=4 flux=113 on 1600
