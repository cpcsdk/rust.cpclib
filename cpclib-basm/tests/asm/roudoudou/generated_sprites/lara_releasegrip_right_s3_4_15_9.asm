ex hl,de
;*** line 1/64 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #30 : ld h,a
;*** line 13/64 ***
ld bc,#55
ld de,#AA80
ld a,l : add 5 : ld l,a : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : dec l : dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 14/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 15/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
dec l : dec l : dec l : dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 17/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 18/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C4
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 19/64 ***
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 20/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 21/64 ***
inc l : inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),e
inc l : ld (hl),#90
inc l : ld (hl),b
set 3,h
;*** line 22/64 ***
ld (hl),b
dec l : ld (hl),#1A
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 23/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#50
inc l : ld (hl),b
res 3,h
;*** line 24/64 ***
ld (hl),e
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#41
inc l : ld (hl),#30
inc l : ld (hl),b
set 3,h
;*** line 26/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#28
dec l : ld (hl),#F
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 27/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#50
inc l : ld (hl),#F
inc l : ld (hl),#A
res 3,h
;*** line 28/64 ***
ld (hl),#A
dec l : ld (hl),#F
dec l : ld (hl),#50
dec l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 29/64 ***
ld (hl),#10
inc l : ld (hl),#4E
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 30/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#1A
dec l : ld (hl),#41
res 4,h
;*** line 31/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#E0
inc l : ld (hl),#8D
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 32/64 ***
ld (hl),#A
dec l : ld (hl),#F
dec l : ld (hl),#C0
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
ld (hl),#41
inc l : ld (hl),#1A
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 34/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#87
dec l : ld (hl),#50
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 35/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#E8
inc l : ld (hl),#D4
inc l : ld a,(hl) : and c : or #20 :ld (hl),a
res 3,h
;*** line 36/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#AD
dec l : ld (hl),#32
dec l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 37/64 ***
dec l : ld (hl),b
inc l : ld (hl),#26
inc l : ld (hl),#AD
inc l : ld (hl),#E0
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 38/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#78
dec l : ld (hl),#7
dec l : ld (hl),#60
dec l : ld (hl),#44
res 4,h
;*** line 39/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#D4
inc l : ld (hl),#E9
inc l : ld (hl),#A0
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 40/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#54
dec l : ld (hl),#E8
dec l : ld (hl),#4
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld (hl),#40
inc l : ld (hl),#98
inc l : ld (hl),#8D
inc l : ld (hl),#5A
inc l : ld (hl),#A0
set 3,h
;*** line 42/64 ***
inc l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),#5A
dec l : ld (hl),#C9
dec l : ld (hl),#CC
dec l : ld (hl),#40
set 4,h
;*** line 43/64 ***
ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#CC
inc l : ld (hl),#1A
inc l : ld (hl),b
inc l : ld (hl),e
res 3,h
;*** line 44/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#E0
dec l : ld (hl),#A
dec l : ld (hl),#C9
dec l : ld (hl),#CC
dec l : ld (hl),#40
set 5,h
;*** line 45/64 ***
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#8D
inc l : ld (hl),#A0
set 3,h
;*** line 46/64 ***
ld (hl),e
dec l : ld (hl),#CC
dec l : ld (hl),b
res 4,h
;*** line 47/64 ***
dec l : ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#8D
inc l : ld (hl),#28
res 3,h
;*** line 48/64 ***
inc l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),#87
dec l : ld (hl),#CC
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
inc l : inc l : ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),b
set 3,h
;*** line 50/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#40
set 4,h
;*** line 51/64 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#88
res 3,h
;*** line 52/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
set 5,h
;*** line 53/64 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#90
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 54/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#F
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 56/64 ***
ld (hl),b
dec l : ld (hl),#30
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
ld (hl),b
inc l : ld (hl),e
set 3,h
;*** line 58/64 ***
ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 59/64 ***
ld (hl),#40
inc l : ld (hl),b
res 3,h
;*** line 60/64 ***
ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 61/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
set 3,h
;*** line 62/64 ***
ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/64 ***
dec l : ld (hl),b
inc l : ld (hl),e
res 3,h
;*** line 64/64 ***
ld (hl),e
dec l : ld (hl),b
ret
; #DBG curx=3 flux=263 on 704
