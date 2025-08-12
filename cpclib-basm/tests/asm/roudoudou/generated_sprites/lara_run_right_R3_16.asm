ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #18 : ld h,a
;*** line 11/64 ***
ld bc,#55
ld de,#40C0
ld a,l : add 11 : ld l,a : ld a,(hl) : and #AA : ld (hl),a
res 3,h
set 5,h
;*** line 13/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 14/64 ***
ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 15/64 ***
ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 16/64 ***
ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 18/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 19/64 ***
ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 20/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 5,h
set 3,h
res 4,h
;*** line 23/64 ***
ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 24/64 ***
ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 26/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 27/64 ***
ld (hl),#45
res 3,h
;*** line 28/64 ***
ld (hl),#10
set 5,h
;*** line 29/64 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#25
set 3,h
;*** line 30/64 ***
ld (hl),#1A
dec l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 31/64 ***
inc l : ld (hl),#5
res 3,h
;*** line 32/64 ***
ld (hl),#5
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
dec l : ld (hl),#44
inc l : ld (hl),#1A
set 3,h
;*** line 34/64 ***
ld (hl),#1A
dec l : ld (hl),d
set 4,h
;*** line 35/64 ***
ld (hl),d
inc l : ld (hl),#80
res 3,h
;*** line 36/64 ***
ld (hl),#8A
dec l : ld (hl),#50
set 5,h
set 3,h
;*** line 38/64 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
res 3,h
;*** line 40/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 42/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 43/64 ***
ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 44/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 45/64 ***
dec l : dec l : ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),e
set 3,h
;*** line 46/64 ***
ld (hl),e
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 47/64 ***
inc l : inc l : ld (hl),b
inc l : ld (hl),e
res 3,h
;*** line 48/64 ***
ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
dec l : dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),e
set 3,h
;*** line 50/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 51/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#80
res 3,h
;*** line 52/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 53/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
res 4,h
;*** line 55/64 ***
ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 56/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#80
;*** line 57/64 ***
;*** line 58/64 ***
;*** line 59/64 ***
;*** line 60/64 ***
;*** line 61/64 ***
;*** line 62/64 ***
;*** line 63/64 ***
;*** line 64/64 ***
ret
; #DBG curx=16 flux=97 on 1600
