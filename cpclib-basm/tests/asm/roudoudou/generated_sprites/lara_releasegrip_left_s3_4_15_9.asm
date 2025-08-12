ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #30 : ld h,a
;*** line 13/64 ***
ld bc,#AA
ld de,#5540
inc l : inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : dec l : dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 14/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 15/64 ***
dec l : dec l : dec l : dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 17/64 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 18/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld (hl),#C8
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 19/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 20/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 21/64 ***
ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 22/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#61
dec l : ld (hl),b
res 4,h
;*** line 23/64 ***
ld (hl),b
inc l : ld (hl),#A0
inc l : ld (hl),#C0
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 24/64 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),#28
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 26/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#1E
dec l : ld (hl),#10
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 27/64 ***
inc l : ld (hl),#5
inc l : ld (hl),#F
inc l : ld (hl),#A0
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 28/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#E0
dec l : ld (hl),#F
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 29/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#85
inc l : ld (hl),#20
set 3,h
;*** line 30/64 ***
ld (hl),#28
dec l : ld (hl),#25
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 31/64 ***
dec l : ld (hl),e
inc l : ld (hl),#1A
inc l : ld (hl),#D0
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 32/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#F
dec l : ld (hl),#41
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#25
inc l : ld (hl),#82
set 3,h
;*** line 34/64 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#4B
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 35/64 ***
ld a,(hl) : and c : or #10 :ld (hl),a
inc l : ld (hl),#5E
inc l : ld (hl),#7C
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 36/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#93
dec l : ld (hl),#F
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 37/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#50
inc l : ld (hl),#5E
inc l : ld (hl),#33
inc l : ld (hl),b
set 3,h
;*** line 38/64 ***
ld (hl),#88
dec l : ld (hl),#10
dec l : ld (hl),#F
dec l : ld (hl),#B4
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 39/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#5E
inc l : ld (hl),#A8
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 40/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#7C
dec l : ld (hl),#AD
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld (hl),e
inc l : ld (hl),#B4
inc l : ld (hl),#1A
inc l : ld (hl),#CC
inc l : ld (hl),#88
set 3,h
;*** line 42/64 ***
ld (hl),#80
dec l : ld (hl),#CC
dec l : ld (hl),#4E
dec l : ld (hl),#A5
dec l : ld (hl),#50
dec l : ld (hl),b
set 4,h
;*** line 43/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#41
inc l : ld (hl),#82
inc l : ld (hl),#5
inc l : ld (hl),#6C
inc l : ld (hl),#C8
inc l : ld (hl),b
res 3,h
;*** line 44/64 ***
ld (hl),#80
dec l : ld (hl),#CC
dec l : ld (hl),#C6
dec l : ld (hl),#5
dec l : ld (hl),#5A
dec l : ld (hl),#10
set 5,h
;*** line 45/64 ***
inc l : ld (hl),e
inc l : ld (hl),#C4
inc l : ld (hl),#98
inc l : ld (hl),b
set 3,h
;*** line 46/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#80
dec l : ld (hl),#C4
dec l : ld (hl),e
res 4,h
;*** line 47/64 ***
ld (hl),e
inc l : ld (hl),#C4
inc l : ld (hl),#5E
inc l : ld (hl),#80
res 3,h
;*** line 48/64 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#1A
dec l : ld (hl),#90
dec l : ld (hl),e
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),b
set 3,h
;*** line 50/64 ***
ld (hl),b
dec l : ld (hl),#C8
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 51/64 ***
inc l : ld (hl),#44
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/64 ***
dec l : ld (hl),#80
dec l : ld (hl),#C4
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 53/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 54/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 56/64 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
inc l : ld (hl),#44
inc l : ld (hl),b
set 3,h
;*** line 58/64 ***
ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 59/64 ***
ld (hl),b
inc l : ld (hl),#80
res 3,h
;*** line 60/64 ***
ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 61/64 ***
ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 62/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/64 ***
ld (hl),e
inc l : ld (hl),b
res 3,h
;*** line 64/64 ***
ld (hl),b
dec l : ld (hl),e
ret
; #DBG curx=6 flux=264 on 704
