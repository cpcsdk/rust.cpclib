ex hl,de
;*** line 1/88 ***
ld a,h : add 48 : ld h,a
;*** line 5/88 ***
ld bc,#AA
ld de,#5505
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#50
set 3,h
;*** line 6/88 ***
ld (hl),b
inc l : ld (hl),#C0
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 7/88 ***
dec l : ld (hl),b
dec l : ld (hl),#14
res 3,h
;*** line 8/88 ***
ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/88 ***
inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 10/88 ***
inc l : ld (hl),e
inc l : ld (hl),#20
inc l : ld (hl),#C0
inc l : ld (hl),b
set 4,h
;*** line 11/88 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),#20
dec l : ld (hl),e
res 3,h
;*** line 12/88 ***
ld (hl),e
inc l : ld (hl),#20
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 13/88 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),#92
dec l : ld (hl),#14
set 3,h
;*** line 14/88 ***
ld (hl),#40
inc l : ld (hl),#F
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),b
res 4,h
;*** line 15/88 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#F
dec l : ld (hl),e
res 3,h
;*** line 16/88 ***
ld (hl),e
inc l : ld (hl),#30
inc l : ld (hl),#20
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/88 ***
ld (hl),b
dec l : ld (hl),#C4
dec l : ld (hl),#C0
dec l : ld (hl),#F
dec l : ld (hl),b
set 3,h
;*** line 18/88 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#40
inc l : ld (hl),#C4
inc l : ld (hl),#80
set 4,h
;*** line 19/88 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#4B
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 20/88 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#40
inc l : ld (hl),#C4
inc l : ld (hl),#80
set 5,h
;*** line 21/88 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#A
dec l : ld (hl),#41
dec l : ld (hl),b
set 3,h
;*** line 22/88 ***
ld (hl),e
inc l : ld (hl),#F
inc l : ld (hl),#A
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 23/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#A
dec l : ld (hl),b
res 3,h
;*** line 24/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#4A
inc l : ld (hl),#80
inc l : ld (hl),#40
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/88 ***
ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#A
dec l : ld (hl),#5A
dec l : ld (hl),e
set 3,h
;*** line 26/88 ***
ld (hl),#54
inc l : ld (hl),#F8
inc l : ld (hl),#28
inc l : inc l : ld (hl),b
inc l : ld (hl),#80
set 4,h
;*** line 27/88 ***
ld (hl),b
dec l : ld (hl),b
dec l : dec l : ld (hl),#80
dec l : ld (hl),#F8
dec l : ld (hl),#4
res 3,h
;*** line 28/88 ***
ld (hl),#54
inc l : ld (hl),#70
inc l : ld (hl),#A0
inc l : inc l : ld (hl),b
inc l : ld (hl),#80
set 5,h
;*** line 29/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AC
dec l : ld (hl),e
set 3,h
;*** line 30/88 ***
ld (hl),e
inc l : ld (hl),#AC
inc l : ld a,(hl) : and d : ld (hl),a
inc l : inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 31/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#FC
dec l : ld (hl),e
res 3,h
;*** line 32/88 ***
ld (hl),#54
inc l : ld (hl),#EC
inc l : ld a,(hl) : and d : ld (hl),a
inc l : inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#B9
dec l : ld (hl),e
set 3,h
;*** line 34/88 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld a,(hl) : and d : ld (hl),a
inc l : inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 35/88 ***
dec l : dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#44
res 3,h
;*** line 36/88 ***
ld (hl),#54
inc l : ld (hl),#FC
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 37/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
set 3,h
;*** line 38/88 ***
ld (hl),#44
inc l : ld (hl),#C8
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 39/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
res 3,h
;*** line 40/88 ***
ld (hl),#44
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
set 3,h
;*** line 42/88 ***
ld (hl),e
inc l : ld (hl),#F
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 43/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#1A
dec l : ld (hl),e
res 3,h
;*** line 44/88 ***
ld (hl),e
inc l : ld (hl),#74
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 45/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),b
set 3,h
;*** line 46/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C8
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 47/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
res 3,h
;*** line 48/88 ***
ld (hl),#44
inc l : ld (hl),#C8
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/88 ***
ld (hl),b
dec l : ld (hl),#C8
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 50/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#80
set 4,h
;*** line 51/88 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 52/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),#88
set 5,h
;*** line 53/88 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#54
set 3,h
;*** line 54/88 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#10
res 3,h
;*** line 56/88 ***
ld (hl),#40
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 58/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 59/88 ***
ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 60/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 61/88 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 62/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 63/88 ***
ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/88 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/88 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 66/88 ***
inc l : ld a,(hl) : and c : ld (hl),a
;*** line 67/88 ***
;*** line 68/88 ***
;*** line 69/88 ***
;*** line 70/88 ***
;*** line 71/88 ***
;*** line 72/88 ***
;*** line 73/88 ***
;*** line 74/88 ***
;*** line 75/88 ***
;*** line 76/88 ***
;*** line 77/88 ***
;*** line 78/88 ***
;*** line 79/88 ***
;*** line 80/88 ***
;*** line 81/88 ***
;*** line 82/88 ***
;*** line 83/88 ***
;*** line 84/88 ***
;*** line 85/88 ***
;*** line 86/88 ***
;*** line 87/88 ***
;*** line 88/88 ***
ret
; #DBG curx=3 flux=296 on 616
