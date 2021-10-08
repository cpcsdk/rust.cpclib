ex hl,de
;*** line 1/88 ***
ld a,l : add 128 : ld l,a : ld a,h : adc #0 : ld h,a
;*** line 17/88 ***
ld bc,#AA
ld de,#555
inc l : inc l : inc l : ld (hl),b
set 3,h
;*** line 18/88 ***
ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 19/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),d
res 3,h
;*** line 20/88 ***
ld (hl),d
inc l : ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 21/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),d
set 3,h
;*** line 22/88 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),b
res 4,h
;*** line 23/88 ***
ld a,(hl) : and e : or #20 :ld (hl),a
dec l : ld (hl),d
res 3,h
;*** line 24/88 ***
ld (hl),d
inc l : ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/88 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),d
dec l : ld (hl),b
set 3,h
;*** line 26/88 ***
ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),b
set 4,h
;*** line 27/88 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#85
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 28/88 ***
inc l : ld (hl),#40
inc l : ld (hl),#85
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 29/88 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A
dec l : ld (hl),#E0
dec l : ld (hl),#8D
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 30/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),#34
inc l : ld (hl),#A
inc l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 31/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#E0
dec l : ld (hl),#85
dec l : ld (hl),#40
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 32/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),#8D
inc l : ld (hl),#88
inc l : ld (hl),#80
inc l : ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#20
dec l : ld (hl),#9C
dec l : ld (hl),#85
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 34/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),#E1
inc l : ld (hl),#20
set 4,h
;*** line 35/88 ***
ld (hl),#A
dec l : ld (hl),#F
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 36/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#F
inc l : ld (hl),#A
set 5,h
;*** line 37/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#10
dec l : ld (hl),#B4
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 38/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#F
inc l : ld (hl),b
res 4,h
;*** line 39/88 ***
inc l : ld (hl),#20
dec l : ld (hl),#2D
dec l : ld (hl),#85
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 40/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#30
inc l : ld (hl),#F
inc l : ld (hl),#A
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/88 ***
dec l : ld (hl),#20
dec l : ld (hl),#F
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 42/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#70
inc l : ld (hl),#20
set 4,h
;*** line 43/88 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#F0
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 44/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#F0
inc l : ld (hl),#A
inc l : ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 45/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),#54
set 3,h
;*** line 46/88 ***
ld (hl),#5C
inc l : ld (hl),#A
res 4,h
;*** line 47/88 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#FC
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 48/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#D0
inc l : ld (hl),#FC
inc l : ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/88 ***
dec l : ld (hl),#A
dec l : ld (hl),#FC
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 50/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#FC
inc l : ld (hl),#A
set 4,h
;*** line 51/88 ***
ld (hl),b
dec l : ld (hl),#E8
dec l : ld (hl),#44
res 3,h
;*** line 52/88 ***
ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),#A
set 5,h
;*** line 53/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#40
set 3,h
;*** line 54/88 ***
ld (hl),#40
inc l : ld (hl),#CC
inc l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 55/88 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),#40
res 3,h
;*** line 56/88 ***
ld (hl),#44
inc l : ld (hl),#DC
inc l : ld (hl),#20
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#40
set 3,h
;*** line 58/88 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 59/88 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 60/88 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),b
set 5,h
;*** line 61/88 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 62/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 63/88 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),b
set 3,h
;*** line 66/88 ***
ld (hl),#40
inc l : ld (hl),#CC
inc l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 67/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#40
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 68/88 ***
inc l : ld (hl),#40
inc l : ld (hl),#CC
inc l : ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 69/88 ***
dec l : ld (hl),#88
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 70/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),b
res 4,h
;*** line 71/88 ***
ld (hl),#88
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 72/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),#88
inc l : ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 73/88 ***
dec l : ld (hl),#20
dec l : ld (hl),#D
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 74/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),d
inc l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 75/88 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 76/88 ***
ld (hl),b
inc l : ld (hl),#40
inc l : ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 77/88 ***
dec l : ld (hl),#80
dec l : ld (hl),b
set 3,h
;*** line 78/88 ***
ld (hl),b
inc l : ld (hl),#80
res 4,h
;*** line 79/88 ***
ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 80/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 81/88 ***
dec l : ld (hl),#80
dec l : ld (hl),b
set 3,h
;*** line 82/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
set 4,h
;*** line 83/88 ***
inc l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
res 3,h
;*** line 84/88 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
ld a,h : add 16 : ld h,a
;*** line 88/88 ***
dec l : ld a,(hl) : and c : ld (hl),a
ret
; #DBG curx=5 flux=298 on 968
