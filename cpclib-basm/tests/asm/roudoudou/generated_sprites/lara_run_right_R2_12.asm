ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #30 : ld h,a
;*** line 13/64 ***
ld bc,#55
ld de,#4080
ld a,l : add 7 : ld l,a : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 14/64 ***
ld (hl),b
ld a,l : add 64 : ld l,a : ld a,h : adc #C8 : ld h,a
;*** line 17/64 ***
ld (hl),d
set 3,h
;*** line 18/64 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
set 4,h
;*** line 19/64 ***
ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 20/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
set 5,h
;*** line 21/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 22/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 23/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 24/64 ***
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add 64 : ld l,a : ld a,h : adc #10 : ld h,a
;*** line 29/64 ***
ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 30/64 ***
ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 31/64 ***
ld a,(hl) : and #AA : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 33/64 ***
ld a,(hl) : and #AA : or d :ld (hl),a
set 3,h
;*** line 34/64 ***
ld (hl),#50
set 4,h
;*** line 35/64 ***
ld (hl),#50
res 3,h
;*** line 36/64 ***
ld (hl),#50
set 5,h
;*** line 37/64 ***
dec l : ld (hl),d
inc l : ld (hl),e
set 3,h
;*** line 38/64 ***
ld (hl),e
dec l : ld (hl),d
res 4,h
;*** line 39/64 ***
ld (hl),d
inc l : ld (hl),e
res 3,h
;*** line 40/64 ***
ld (hl),#50
dec l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),e
set 3,h
;*** line 42/64 ***
ld (hl),b
ld a,l : add 64 : ld l,a : ld a,h : adc #0 : ld h,a
;*** line 50/64 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 51/64 ***
ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 52/64 ***
ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 53/64 ***
ld (hl),b
dec l : ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 54/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
res 4,h
;*** line 55/64 ***
inc l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 56/64 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),d
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld a,(hl) : and #AA : ld (hl),a
;*** line 58/64 ***
;*** line 59/64 ***
;*** line 60/64 ***
;*** line 61/64 ***
;*** line 62/64 ***
;*** line 63/64 ***
;*** line 64/64 ***
ret
; #DBG curx=12 flux=78 on 1600
