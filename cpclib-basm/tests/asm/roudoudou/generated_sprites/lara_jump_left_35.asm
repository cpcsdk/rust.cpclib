ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #8 : ld h,a
;*** line 10/72 ***
ld bc,#AA
ld de,#5544
ld a,l : sub 14 : ld l,a : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 11/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 12/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 13/72 ***
dec l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 14/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 15/72 ***
inc l : ld (hl),#40
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#80
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 16/72 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#C0
inc l : ld (hl),#40
inc l : ld (hl),b
set 3,h
;*** line 18/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#4A
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 19/72 ***
inc l : ld (hl),b
inc l : ld (hl),#34
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 20/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#4A
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 21/72 ***
inc l : ld (hl),e
inc l : ld (hl),#F
inc l : ld (hl),#C2
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 22/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#E0
dec l : ld (hl),#F
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 23/72 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#60
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 24/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#2D
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#F
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 26/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 27/72 ***
inc l : ld (hl),b
inc l : ld (hl),#A0
inc l : inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 28/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A0
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 29/72 ***
dec l : ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#4E
set 3,h
;*** line 30/72 ***
ld (hl),#4A
dec l : ld (hl),#1A
dec l : ld (hl),#54
res 4,h
;*** line 31/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#4B
inc l : ld (hl),#A
inc l : inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 32/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : dec l : ld (hl),#28
dec l : ld (hl),#10
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/72 ***
dec l : ld (hl),#54
inc l : ld (hl),#B8
inc l : ld (hl),#82
set 3,h
;*** line 34/72 ***
ld (hl),#A
dec l : ld (hl),#5C
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 35/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5E
inc l : ld (hl),#4E
res 3,h
;*** line 36/72 ***
ld (hl),#4E
dec l : ld (hl),#5E
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 37/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#11
inc l : ld (hl),#1E
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 38/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#40
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 39/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5E
inc l : ld (hl),#1A
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 40/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#4E
dec l : ld (hl),#5E
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#74
inc l : ld (hl),#AD
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 42/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#25
dec l : ld (hl),#C4
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 43/72 ***
inc l : ld (hl),e
inc l : ld (hl),#9C
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 44/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#87
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 45/72 ***
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 46/72 ***
ld (hl),#80
dec l : ld (hl),#C0
dec l : ld (hl),#5
res 4,h
;*** line 47/72 ***
ld (hl),e
inc l : ld (hl),#40
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 48/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#98
dec l : ld (hl),e
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld (hl),#5
inc l : ld (hl),#70
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 50/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C6
dec l : ld (hl),#5
set 4,h
;*** line 51/72 ***
ld (hl),e
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),e
set 5,h
;*** line 53/72 ***
ld (hl),b
inc l : ld (hl),#C8
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 54/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),b
res 4,h
;*** line 55/72 ***
ld (hl),e
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 56/72 ***
dec l : ld (hl),#88
dec l : ld (hl),e
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 58/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 59/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 61/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 62/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/72 ***
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
ld (hl),b
inc l : ld (hl),#80
res 3,h
;*** line 68/72 ***
ld (hl),#80
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 69/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 70/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 71/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#80
res 3,h
;*** line 72/72 ***
ld (hl),b
dec l : ld (hl),#40
dec l : ld a,(hl) : and c : ld (hl),a
ret
; #DBG curx=3 flux=317 on 1800
