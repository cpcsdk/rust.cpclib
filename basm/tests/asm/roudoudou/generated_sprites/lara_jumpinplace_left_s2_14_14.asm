ex hl,de
;*** line 1/72 ***
ld bc,#AA
ld de,#5540
dec l : ld (hl),#10
inc l : ld (hl),#A
inc l : ld (hl),#C0
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 2/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),#4A
dec l : ld (hl),#10
set 4,h
;*** line 3/72 ***
ld (hl),#25
inc l : ld (hl),#4A
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 4/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#4A
dec l : ld (hl),#5
set 5,h
;*** line 5/72 ***
ld (hl),#10
inc l : ld (hl),#E1
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 6/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#F
dec l : ld (hl),b
res 4,h
;*** line 7/72 ***
ld (hl),#5
inc l : ld (hl),#3C
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 8/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#1A
dec l : ld (hl),#5
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 10/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#B0
dec l : ld (hl),#F
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 11/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#FC
inc l : ld (hl),#F0
inc l : ld (hl),b
inc l : ld (hl),#80
res 3,h
;*** line 12/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),#F0
dec l : ld (hl),#B
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 13/72 ***
inc l : ld (hl),#5C
inc l : ld (hl),#A8
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
set 3,h
;*** line 14/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),#5
res 4,h
;*** line 15/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5C
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 16/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#E0
dec l : ld (hl),#C
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/72 ***
inc l : ld (hl),#5
inc l : ld (hl),#A8
set 3,h
;*** line 18/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),#5
set 4,h
;*** line 19/72 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#D4
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 20/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#54
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 21/72 ***
inc l : ld (hl),e
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 22/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),b
res 4,h
;*** line 23/72 ***
ld (hl),#44
inc l : ld (hl),#CC
inc l : ld (hl),b
res 3,h
;*** line 24/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#64
dec l : ld (hl),#5C
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),b
set 3,h
;*** line 26/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),b
set 4,h
;*** line 27/72 ***
ld (hl),b
inc l : ld (hl),#1A
inc l : ld (hl),#20
res 3,h
;*** line 28/72 ***
ld (hl),#A8
dec l : ld (hl),#25
dec l : ld (hl),b
set 5,h
;*** line 29/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#80
set 3,h
;*** line 30/72 ***
ld (hl),#80
dec l : ld (hl),#C4
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 31/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#80
res 3,h
;*** line 32/72 ***
ld (hl),#20
dec l : ld (hl),#F
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),#88
set 3,h
;*** line 34/72 ***
ld (hl),#88
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 35/72 ***
inc l : ld (hl),e
inc l : ld (hl),#C8
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 36/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#C0
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 37/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 38/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 39/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 40/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#44
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 42/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
set 4,h
;*** line 43/72 ***
ld (hl),e
inc l : ld (hl),b
res 3,h
;*** line 44/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
set 5,h
;*** line 45/72 ***
ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 46/72 ***
ld (hl),b
dec l : ld (hl),e
res 4,h
;*** line 47/72 ***
ld (hl),e
inc l : ld (hl),b
res 3,h
;*** line 48/72 ***
ld (hl),b
dec l : ld (hl),e
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld (hl),e
inc l : ld (hl),b
set 3,h
;*** line 50/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
set 4,h
;*** line 51/72 ***
ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
;*** line 53/72 ***
;*** line 54/72 ***
;*** line 55/72 ***
;*** line 56/72 ***
;*** line 57/72 ***
;*** line 58/72 ***
;*** line 59/72 ***
;*** line 60/72 ***
;*** line 61/72 ***
;*** line 62/72 ***
;*** line 63/72 ***
;*** line 64/72 ***
;*** line 65/72 ***
;*** line 66/72 ***
;*** line 67/72 ***
;*** line 68/72 ***
;*** line 69/72 ***
;*** line 70/72 ***
;*** line 71/72 ***
;*** line 72/72 ***
ret
; #DBG curx=5 flux=220 on 792
