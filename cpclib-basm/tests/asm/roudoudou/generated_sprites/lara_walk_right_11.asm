ex hl,de
;*** line 1/64 ***
set 3,h
;*** line 2/64 ***
ld bc,#AA
ld de,#4055
inc l : inc l : inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
set 4,h
;*** line 3/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 4/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 5/64 ***
ld (hl),d
inc l : ld (hl),#CC
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 6/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),d
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 7/64 ***
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 8/64 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/64 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),#80
inc l : ld (hl),#A0
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 10/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),#C0
dec l : ld (hl),d
dec l : ld (hl),#80
dec l : ld (hl),d
set 4,h
;*** line 11/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#41
inc l : ld (hl),#28
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 12/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),#C0
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),d
set 5,h
;*** line 13/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),#F
inc l : ld (hl),#A
set 3,h
;*** line 14/64 ***
ld (hl),#A
dec l : ld (hl),#F
dec l : ld (hl),#50
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 15/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#F
inc l : ld (hl),#A
res 3,h
;*** line 16/64 ***
ld (hl),#20
dec l : ld (hl),#8D
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#2D
inc l : ld (hl),#20
set 3,h
;*** line 18/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#B0
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),d
set 4,h
;*** line 19/64 ***
ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#A0
res 3,h
;*** line 20/64 ***
ld (hl),#A0
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
set 5,h
;*** line 21/64 ***
inc l : inc l : ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#20
set 3,h
;*** line 22/64 ***
ld (hl),#A
dec l : ld (hl),#5E
dec l : ld (hl),#90
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 23/64 ***
inc l : ld (hl),#14
inc l : ld (hl),#1A
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 24/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#1A
dec l : ld (hl),#10
dec l : dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#E0
inc l : ld (hl),#5E
inc l : ld (hl),#A8
set 3,h
;*** line 26/64 ***
ld (hl),#A8
dec l : ld (hl),#84
dec l : ld (hl),#A0
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 27/64 ***
ld (hl),d
inc l : ld (hl),#84
inc l : ld (hl),#25
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 28/64 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#A4
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 29/64 ***
ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),#87
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 30/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#2D
dec l : ld (hl),#31
dec l : ld (hl),b
res 4,h
;*** line 31/64 ***
ld (hl),d
inc l : ld (hl),#54
inc l : ld (hl),#92
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 32/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#87
dec l : ld (hl),#DC
dec l : ld (hl),d
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#2D
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 34/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#8D
dec l : ld (hl),#CC
dec l : ld (hl),b
set 4,h
;*** line 35/64 ***
ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#9C
inc l : ld (hl),#20
res 3,h
;*** line 36/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#8D
dec l : ld (hl),#C4
dec l : ld (hl),b
set 5,h
;*** line 37/64 ***
ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#98
inc l : ld (hl),#A
set 3,h
;*** line 38/64 ***
ld (hl),#28
dec l : ld (hl),#98
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 39/64 ***
ld (hl),#50
inc l : ld (hl),#C4
inc l : ld (hl),#C8
inc l : ld (hl),#A
res 3,h
;*** line 40/64 ***
ld (hl),#A
dec l : ld (hl),#D8
dec l : ld (hl),#C4
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#99
inc l : ld (hl),#80
set 3,h
;*** line 42/64 ***
ld (hl),#80
dec l : ld (hl),#DC
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 43/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),d
inc l : ld (hl),#80
res 3,h
;*** line 44/64 ***
ld (hl),#80
dec l : ld (hl),#99
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 45/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),d
inc l : ld (hl),#80
set 3,h
;*** line 46/64 ***
ld (hl),#80
dec l : ld (hl),d
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 47/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),d
inc l : ld (hl),#80
res 3,h
;*** line 48/64 ***
ld (hl),#80
dec l : ld (hl),d
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#C0
inc l : ld (hl),b
set 3,h
;*** line 50/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#C0
dec l : ld (hl),#88
dec l : ld (hl),d
set 4,h
;*** line 51/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),d
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 52/64 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#C0
dec l : ld (hl),#88
dec l : ld (hl),d
set 5,h
;*** line 53/64 ***
ld (hl),b
inc l : ld (hl),#20
inc l : ld (hl),b
set 3,h
;*** line 54/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 55/64 ***
ld (hl),#5
inc l : ld (hl),#A
inc l : ld (hl),b
res 3,h
;*** line 56/64 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#20
dec l : ld (hl),#10
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 58/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 59/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 60/64 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 61/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 62/64 ***
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/64 ***
ld (hl),b
inc l : ld (hl),#80
res 3,h
;*** line 64/64 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),d
ret
; #DBG curx=6 flux=301 on 960
