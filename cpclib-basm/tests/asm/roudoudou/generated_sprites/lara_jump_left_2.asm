ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #30 : ld h,a
;*** line 13/72 ***
ld bc,#55
ld de,#80AA
inc l : inc l : inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 14/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 15/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 16/72 ***
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/72 ***
inc l : inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 18/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),#44
inc l : ld (hl),#C4
inc l : ld (hl),#88
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 19/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#C8
dec l : ld (hl),#10
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 20/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),#40
inc l : ld (hl),#C8
inc l : ld (hl),#C0
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 21/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#6C
dec l : ld (hl),#40
set 3,h
;*** line 22/72 ***
ld (hl),#5
inc l : ld (hl),#4B
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 23/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#60
dec l : ld (hl),#10
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 24/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),#C2
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),d
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#F
dec l : ld (hl),#5
set 3,h
;*** line 26/72 ***
ld (hl),#5
inc l : ld (hl),#F
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 27/72 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#68
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 28/72 ***
ld (hl),#10
inc l : ld (hl),#4B
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 29/72 ***
ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#20
dec l : ld (hl),#14
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 30/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#8D
inc l : ld (hl),#A
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
res 4,h
;*** line 31/72 ***
ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#40
res 3,h
;*** line 32/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),d
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : dec l : ld (hl),#A
dec l : ld (hl),#F
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 34/72 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#A
inc l : inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 35/72 ***
dec l : dec l : ld (hl),d
dec l : ld (hl),#AD
dec l : ld (hl),#54
res 3,h
;*** line 36/72 ***
ld (hl),#54
inc l : ld (hl),#4B
inc l : ld (hl),#A0
set 5,h
;*** line 37/72 ***
ld (hl),#A8
dec l : ld (hl),#F
dec l : ld (hl),#50
set 3,h
;*** line 38/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#94
inc l : ld (hl),#5E
inc l : ld (hl),#A8
res 4,h
;*** line 39/72 ***
ld (hl),#A8
dec l : ld (hl),#2D
dec l : ld (hl),#40
res 3,h
;*** line 40/72 ***
ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),#A8
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),#1A
dec l : ld (hl),#85
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 42/72 ***
ld (hl),b
inc l : ld (hl),#A5
inc l : ld (hl),#D6
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 43/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#64
dec l : ld (hl),#11
dec l : ld (hl),#1A
dec l : ld (hl),#C8
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 44/72 ***
inc l : ld (hl),#5
inc l : ld (hl),#87
inc l : ld (hl),#C0
inc l : ld (hl),#90
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 45/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#CC
dec l : ld (hl),b
dec l : ld (hl),#40
set 3,h
;*** line 46/72 ***
ld (hl),#50
inc l : ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#88
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 47/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld (hl),#40
res 3,h
;*** line 48/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),d
inc l : ld (hl),#98
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
dec l : ld (hl),#88
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 50/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#25
inc l : ld (hl),#20
set 4,h
;*** line 51/72 ***
ld (hl),#A
dec l : ld (hl),#25
dec l : ld a,(hl) : and e : or #40 :ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#F
inc l : ld (hl),#A
set 5,h
;*** line 53/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 54/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/72 ***
ld (hl),d
dec l : ld (hl),#CC
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 56/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),d
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld (hl),b
dec l : ld (hl),#C4
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 58/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),d
set 4,h
;*** line 59/72 ***
ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 60/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),d
set 5,h
;*** line 61/72 ***
ld (hl),#88
dec l : ld (hl),#44
set 3,h
;*** line 62/72 ***
ld (hl),b
inc l : ld (hl),d
res 4,h
;*** line 63/72 ***
ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 64/72 ***
ld (hl),#40
inc l : ld (hl),#88
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld (hl),d
dec l : ld (hl),b
set 3,h
;*** line 66/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
set 4,h
;*** line 67/72 ***
ld (hl),d
dec l : ld (hl),b
res 3,h
;*** line 68/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
set 5,h
;*** line 69/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 71/72 ***
ld (hl),d
dec l : ld (hl),#C0
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),b
ret
; #DBG curx=21 flux=294 on 1800
