ex hl,de
;*** line 1/64 ***
ld a,h : add 48 : ld h,a
;*** line 5/64 ***
ld bc,#55
ld de,#40AA
ld a,l : add 7 : ld l,a : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 6/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 7/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 8/64 ***
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/64 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),d
set 3,h
;*** line 10/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),#CC
inc l : ld (hl),b
set 4,h
;*** line 11/64 ***
ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),d
res 3,h
;*** line 12/64 ***
ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),d
inc l : ld (hl),#80
set 5,h
;*** line 13/64 ***
ld (hl),#45
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 14/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#CD
res 4,h
;*** line 15/64 ***
ld (hl),#D0
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 16/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#80
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
ld (hl),#F
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 18/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#F
set 4,h
;*** line 19/64 ***
ld (hl),#8F
dec l : ld (hl),#50
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 20/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#F
set 5,h
;*** line 21/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C5
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 22/64 ***
ld (hl),d
inc l : ld (hl),#A5
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 23/64 ***
ld (hl),b
dec l : ld (hl),#D0
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 24/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#50
inc l : ld (hl),#10
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A5
dec l : ld (hl),#D0
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 26/64 ***
ld (hl),d
inc l : ld (hl),#E0
inc l : ld (hl),#A5
inc l : ld (hl),#20
set 4,h
;*** line 27/64 ***
ld (hl),#A
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),#D0
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 28/64 ***
inc l : ld (hl),#50
inc l : ld (hl),#E0
inc l : ld (hl),#E5
inc l : ld (hl),#A
set 5,h
;*** line 29/64 ***
ld (hl),#A
dec l : ld (hl),#25
dec l : ld (hl),#54
dec l : ld (hl),#E0
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 30/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#A0
inc l : ld (hl),#76
inc l : ld (hl),#F
inc l : ld (hl),#F
res 4,h
;*** line 31/64 ***
ld (hl),#A8
dec l : ld (hl),#62
dec l : ld (hl),#54
dec l : ld (hl),#F0
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 32/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#F0
inc l : ld (hl),#4
inc l : ld (hl),b
inc l : ld (hl),#AC
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
ld (hl),#F
dec l : ld (hl),#F
dec l : ld (hl),#D4
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 34/64 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#8
inc l : ld (hl),#4E
inc l : ld (hl),#E0
set 4,h
;*** line 35/64 ***
dec l : ld (hl),#22
dec l : ld (hl),#DC
dec l : ld (hl),#C0
dec l : ld (hl),d
res 3,h
;*** line 36/64 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#31
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 37/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#90
dec l : ld (hl),#CC
dec l : ld (hl),#44
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 38/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#C8
inc l : ld (hl),#76
inc l : ld (hl),#20
res 4,h
;*** line 39/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#CC
dec l : ld (hl),#44
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 40/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#CC
inc l : ld (hl),#20
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
inc l : ld (hl),#8
dec l : ld (hl),#FC
dec l : ld (hl),#C8
dec l : ld (hl),#CC
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 42/64 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#88
inc l : ld (hl),#FC
inc l : ld (hl),#80
set 4,h
;*** line 43/64 ***
ld (hl),#C0
dec l : ld (hl),#48
dec l : ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),#44
res 3,h
;*** line 44/64 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),b
inc l : ld (hl),#AC
inc l : ld (hl),#80
set 5,h
;*** line 45/64 ***
ld (hl),#80
dec l : ld (hl),b
dec l : dec l : ld (hl),#88
dec l : ld (hl),#CC
dec l : ld (hl),#C4
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 46/64 ***
ld (hl),#4
inc l : ld (hl),#64
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
inc l : inc l : ld (hl),b
inc l : ld (hl),#80
res 4,h
;*** line 47/64 ***
ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#CC
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 48/64 ***
inc l : ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),#C0
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld (hl),#80
dec l : ld (hl),b
dec l : dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#4E
dec l : ld (hl),#10
set 3,h
;*** line 50/64 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),#A
inc l : ld (hl),b
inc l : inc l : inc l : ld (hl),b
inc l : ld (hl),#80
set 4,h
;*** line 51/64 ***
ld (hl),#80
dec l : ld a,(hl) : and e : ld (hl),a
dec l : dec l : dec l : dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),b
res 3,h
;*** line 52/64 ***
ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),b
inc l : inc l : inc l : inc l : ld (hl),b
inc l : ld (hl),#80
set 5,h
;*** line 53/64 ***
ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
ld a,l : sub 5 : ld l,a : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
set 3,h
;*** line 54/64 ***
ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add 6 : ld l,a : ld (hl),b
res 4,h
;*** line 55/64 ***
ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
ld a,l : sub 5 : ld l,a : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
res 3,h
;*** line 56/64 ***
ld (hl),d
inc l : ld (hl),b
ld a,l : add 5 : ld l,a : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
ld (hl),b
ld a,l : sub 6 : ld l,a : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
set 3,h
;*** line 58/64 ***
ld a,(hl) : and e : ld (hl),a
ld a,l : add 7 : ld l,a : ld (hl),b
set 4,h
;*** line 59/64 ***
ld (hl),b
res 3,h
;*** line 60/64 ***
ld (hl),b
;*** line 61/64 ***
;*** line 62/64 ***
;*** line 63/64 ***
;*** line 64/64 ***
ret
; #DBG curx=14 flux=298 on 1600