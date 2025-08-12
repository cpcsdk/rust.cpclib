ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #18 : ld h,a
;*** line 11/72 ***
ld bc,#55
ld de,#AA80
ld a,l : sub 13 : ld l,a : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 12/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 13/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 14/72 ***
ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#44
inc l : ld (hl),#C0
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 15/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 16/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/72 ***
inc l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 18/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),b
set 4,h
;*** line 19/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#68
dec l : ld (hl),#10
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 20/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),#C2
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 21/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#60
dec l : ld (hl),#F
dec l : ld (hl),#5
set 3,h
;*** line 22/72 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#C2
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 23/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#4B
dec l : ld (hl),b
res 3,h
;*** line 24/72 ***
ld (hl),#10
inc l : ld (hl),#70
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/72 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),#F
dec l : ld (hl),#10
set 3,h
;*** line 26/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#4B
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
set 4,h
;*** line 27/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),b
res 3,h
;*** line 28/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 29/72 ***
dec l : dec l : ld (hl),#A
dec l : ld (hl),#4B
dec l : ld (hl),#10
set 3,h
;*** line 30/72 ***
ld (hl),#5
inc l : ld (hl),#4E
inc l : ld (hl),#A
res 4,h
;*** line 31/72 ***
inc l : inc l : ld a,(hl) : and c : ld (hl),a
dec l : dec l : ld (hl),#A
dec l : ld (hl),#34
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 32/72 ***
inc l : ld (hl),#41
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/72 ***
dec l : dec l : ld (hl),#A
dec l : ld (hl),#BC
dec l : ld (hl),#54
set 3,h
;*** line 34/72 ***
ld (hl),#54
inc l : ld (hl),#B8
inc l : ld (hl),#A
set 4,h
;*** line 35/72 ***
ld (hl),#A
dec l : ld (hl),#FC
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 36/72 ***
ld (hl),b
inc l : ld (hl),#FC
inc l : ld (hl),#A
set 5,h
;*** line 37/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#4A
dec l : ld (hl),#5E
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 38/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),#4E
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 39/72 ***
dec l : ld (hl),#A
dec l : ld (hl),#5E
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 40/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#5E
inc l : ld (hl),#A
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#4E
dec l : ld (hl),#18
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 42/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#30
inc l : ld (hl),#4E
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 43/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#4A
dec l : ld (hl),#44
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 44/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#4A
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 45/72 ***
dec l : ld (hl),e
dec l : ld (hl),#C8
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 46/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#60
inc l : ld (hl),e
res 4,h
;*** line 47/72 ***
ld (hl),e
dec l : ld (hl),#C8
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 48/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#A
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#64
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 50/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#25
inc l : ld (hl),#A0
set 4,h
;*** line 51/72 ***
ld (hl),#88
dec l : ld (hl),#44
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#88
set 5,h
;*** line 53/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#44
set 3,h
;*** line 54/72 ***
ld (hl),#40
inc l : ld (hl),#C8
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/72 ***
dec l : ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 56/72 ***
ld (hl),#44
inc l : ld (hl),#88
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),b
set 3,h
;*** line 58/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 59/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#CC
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 61/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 62/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#CC
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
set 4,h
;*** line 67/72 ***
ld (hl),e
dec l : ld (hl),b
res 3,h
;*** line 68/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
set 5,h
;*** line 69/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 71/72 ***
ld (hl),e
dec l : ld (hl),#C0
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),b
ret
; #DBG curx=5 flux=301 on 1800
