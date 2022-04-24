ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #18 : ld h,a
;*** line 11/64 ***
ld bc,#55
ld de,#40CC
ld a,l : add 7 : ld l,a : ld a,(hl) : and #AA : ld (hl),a
res 3,h
set 5,h
;*** line 13/64 ***
ld (hl),b
set 3,h
;*** line 14/64 ***
ld (hl),d
res 4,h
;*** line 15/64 ***
ld (hl),b
res 3,h
;*** line 16/64 ***
ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
dec l : ld (hl),b
inc l : ld (hl),d
set 3,h
;*** line 18/64 ***
ld (hl),b
dec l : ld (hl),#44
set 4,h
;*** line 19/64 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 20/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 21/64 ***
ld (hl),b
inc l : ld (hl),#80
inc l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 22/64 ***
ld a,(hl) : and #AA : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 23/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 24/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 26/64 ***
dec l : dec l : ld (hl),b
set 4,h
;*** line 27/64 ***
inc l : inc l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
set 5,h
;*** line 29/64 ***
ld (hl),#F0
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 30/64 ***
ld a,(hl) : and #AA : or d :ld (hl),a
inc l : ld (hl),#F0
res 4,h
;*** line 31/64 ***
ld (hl),d
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 32/64 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
ld (hl),#A0
dec l : ld (hl),#50
set 3,h
;*** line 34/64 ***
dec l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#80
set 4,h
;*** line 35/64 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#C0
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 36/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#D0
inc l : ld (hl),b
set 5,h
;*** line 37/64 ***
ld (hl),#8
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 38/64 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#DC
res 4,h
;*** line 39/64 ***
ld (hl),d
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C0
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 40/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),#11
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld (hl),#C4
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 42/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#C4
set 4,h
;*** line 43/64 ***
ld (hl),#C4
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 44/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#C4
set 5,h
;*** line 45/64 ***
ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 46/64 ***
dec l : dec l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),e
res 4,h
;*** line 47/64 ***
ld (hl),#C4
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 48/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#C4
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 50/64 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),#22
inc l : ld (hl),e
set 4,h
;*** line 51/64 ***
ld (hl),e
dec l : ld (hl),#4A
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 52/64 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),#4A
inc l : ld (hl),e
set 5,h
;*** line 53/64 ***
ld (hl),#88
dec l : ld (hl),b
set 3,h
;*** line 54/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
res 4,h
;*** line 55/64 ***
ld (hl),#C8
dec l : ld (hl),#44
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 56/64 ***
ld (hl),b
inc l : ld (hl),#4A
inc l : ld (hl),e
;*** line 57/64 ***
;*** line 58/64 ***
;*** line 59/64 ***
;*** line 60/64 ***
;*** line 61/64 ***
;*** line 62/64 ***
;*** line 63/64 ***
;*** line 64/64 ***
ret
; #DBG curx=14 flux=147 on 1600
