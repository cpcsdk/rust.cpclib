ex hl,de
;*** line 1/64 ***
ld a,h : add 48 : ld h,a
;*** line 5/64 ***
ld bc,#55
ld de,#AA80
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 6/64 ***
ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add 64 : ld l,a : ld a,h : adc #C8 : ld h,a
;*** line 9/64 ***
inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 10/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 11/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 12/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 13/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 14/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#F0
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 15/64 ***
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 16/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),e
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C3
dec l : ld (hl),e
dec l : ld (hl),b
set 3,h
;*** line 18/64 ***
ld (hl),#30
inc l : ld (hl),#C0
inc l : ld (hl),#F
inc l : ld (hl),e
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 19/64 ***
dec l : ld (hl),b
dec l : ld (hl),#F
dec l : ld (hl),#92
dec l : ld (hl),#40
res 3,h
;*** line 20/64 ***
ld (hl),#C8
inc l : ld (hl),#C1
inc l : ld (hl),#F
inc l : ld (hl),#88
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 21/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#F
dec l : ld (hl),b
set 3,h
;*** line 22/64 ***
ld (hl),#41
inc l : ld (hl),#A5
inc l : ld (hl),#4B
inc l : ld (hl),b
res 4,h
;*** line 23/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#1A
dec l : ld (hl),b
res 3,h
;*** line 24/64 ***
ld (hl),b
inc l : ld (hl),#4A
inc l : ld (hl),#1A
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
ld (hl),#A
dec l : ld (hl),#F
dec l : ld (hl),#AD
dec l : ld (hl),#25
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 26/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#87
inc l : ld (hl),#AD
inc l : ld (hl),#F
inc l : ld (hl),#82
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 27/64 ***
ld (hl),#20
dec l : ld (hl),#D0
dec l : ld (hl),#AC
dec l : ld (hl),#74
dec l : ld (hl),e
dec l : ld (hl),#40
res 3,h
;*** line 28/64 ***
ld (hl),b
inc l : ld (hl),#1A
inc l : ld (hl),#FC
inc l : ld (hl),#FC
inc l : ld (hl),#E1
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 29/64 ***
ld (hl),#82
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),#F
dec l : ld (hl),b
dec l : ld (hl),#1A
set 3,h
;*** line 30/64 ***
ld (hl),#20
inc l : ld (hl),#4
inc l : ld (hl),#B
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#A0
res 4,h
;*** line 31/64 ***
ld (hl),#82
dec l : ld (hl),#40
dec l : ld (hl),#A8
dec l : ld (hl),#F
dec l : ld (hl),b
dec l : ld (hl),#F
res 3,h
;*** line 32/64 ***
ld (hl),#B0
inc l : ld (hl),b
inc l : ld (hl),#FC
inc l : ld (hl),#A8
inc l : ld (hl),#50
inc l : ld (hl),#A
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C0
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 34/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),#25
inc l : ld (hl),#A
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 35/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),#CC
dec l : ld (hl),#44
res 3,h
;*** line 36/64 ***
ld (hl),#44
inc l : ld (hl),#F
inc l : ld (hl),#20
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 37/64 ***
dec l : dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),e
dec l : ld (hl),#44
set 3,h
;*** line 38/64 ***
ld (hl),#44
inc l : ld (hl),e
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 39/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#C0
dec l : ld (hl),#44
res 3,h
;*** line 40/64 ***
ld (hl),#44
inc l : ld (hl),#CC
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
dec l : ld a,(hl) : and c : or #20 :ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#88
dec l : ld (hl),#44
set 3,h
;*** line 42/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),#25
inc l : ld a,(hl) : and c : or #20 :ld (hl),a
set 4,h
;*** line 43/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#44
dec l : ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 44/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),#25
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 45/64 ***
ld (hl),b
dec l : ld (hl),#40
dec l : ld (hl),#88
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 46/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#40
inc l : ld (hl),b
res 4,h
;*** line 47/64 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#44
dec l : ld (hl),#88
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 48/64 ***
ld (hl),b
inc l : ld (hl),#88
inc l : ld (hl),#44
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld (hl),#88
dec l : ld (hl),#44
dec l : ld (hl),#88
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 50/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#88
inc l : ld (hl),#44
inc l : ld (hl),#88
set 4,h
;*** line 51/64 ***
ld (hl),#88
dec l : ld (hl),#44
dec l : ld (hl),#C8
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#C8
inc l : ld (hl),#44
inc l : ld (hl),#88
set 5,h
;*** line 53/64 ***
ld (hl),#88
dec l : ld (hl),b
dec l : ld (hl),#10
set 3,h
;*** line 54/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 55/64 ***
ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#25
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 56/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#FC
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#40
set 3,h
;*** line 58/64 ***
ld (hl),#40
inc l : ld (hl),b
inc l : ld (hl),b
set 4,h
;*** line 59/64 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
set 5,h
;*** line 61/64 ***
dec l : dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
res 4,h
;*** line 63/64 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/64 ***
inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld a,(hl) : and d : ld (hl),a
ret
; #DBG curx=4 flux=315 on 640
