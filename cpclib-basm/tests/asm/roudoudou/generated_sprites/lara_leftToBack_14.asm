ex hl,de
;*** line 1/64 ***
set 3,h
set 4,h
;*** line 3/64 ***
ld bc,#AA
ld de,#400F
inc l : inc l : inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 4/64 ***
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 5/64 ***
inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),b
set 3,h
;*** line 6/64 ***
ld (hl),b
inc l : ld (hl),#80
inc l : ld (hl),d
inc l : ld (hl),b
res 4,h
;*** line 7/64 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 8/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/64 ***
inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 10/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 4,h
;*** line 11/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 12/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 13/64 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 14/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 15/64 ***
inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#88
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 16/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#20
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/64 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 18/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 4,h
;*** line 19/64 ***
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 20/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 21/64 ***
ld a,(hl) : and #55 : or #20 :ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),e
dec l : ld (hl),#5
set 3,h
;*** line 22/64 ***
ld (hl),#5
inc l : ld (hl),e
inc l : ld (hl),e
inc l : ld (hl),#28
res 4,h
;*** line 23/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#5E
dec l : ld (hl),#5
res 3,h
;*** line 24/64 ***
ld (hl),#10
inc l : ld (hl),#60
inc l : ld (hl),#A0
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/64 ***
ld (hl),#28
dec l : ld (hl),#4B
dec l : ld (hl),e
dec l : ld (hl),#61
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 26/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#1A
inc l : ld (hl),#5E
inc l : ld (hl),#1E
inc l : ld (hl),#A
set 4,h
;*** line 27/64 ***
ld (hl),b
dec l : ld (hl),#B9
dec l : ld (hl),#FC
dec l : ld (hl),#A
dec l : ld (hl),b
res 3,h
;*** line 28/64 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#A
inc l : ld (hl),#FC
inc l : ld (hl),#B8
inc l : ld (hl),#80
set 5,h
;*** line 29/64 ***
inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#92
dec l : ld (hl),#AC
dec l : ld (hl),#74
dec l : ld (hl),#20
dec l : ld (hl),#5
set 3,h
;*** line 30/64 ***
ld (hl),#5
inc l : ld (hl),#88
inc l : ld (hl),#FC
inc l : ld (hl),#FC
inc l : ld (hl),#A5
inc l : ld a,(hl) : and #55 : ld (hl),a
res 4,h
;*** line 31/64 ***
dec l : ld (hl),#A
dec l : ld (hl),#B8
dec l : ld (hl),#5C
dec l : ld (hl),#A0
dec l : ld (hl),#41
res 3,h
;*** line 32/64 ***
ld (hl),#10
inc l : ld (hl),#A
inc l : ld (hl),#FC
inc l : ld (hl),#B9
inc l : ld (hl),#80
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/64 ***
inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#5
set 3,h
;*** line 34/64 ***
ld (hl),#41
inc l : ld (hl),b
inc l : ld (hl),#B8
inc l : ld (hl),#30
inc l : ld (hl),#25
inc l : ld a,(hl) : and #55 : ld (hl),a
set 4,h
;*** line 35/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#8D
dec l : ld (hl),#CC
dec l : ld (hl),#CC
dec l : ld (hl),b
dec l : ld (hl),d
res 3,h
;*** line 36/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#64
inc l : ld (hl),#CC
inc l : ld (hl),#8D
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 37/64 ***
ld (hl),#20
dec l : ld (hl),#90
dec l : ld (hl),#C4
dec l : ld (hl),#C8
dec l : ld (hl),#44
dec l : ld (hl),b
set 3,h
;*** line 38/64 ***
ld (hl),b
inc l : ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),d
inc l : ld (hl),#C4
inc l : ld (hl),#A
res 4,h
;*** line 39/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#90
dec l : ld (hl),#CC
dec l : ld (hl),#CC
dec l : ld (hl),#44
dec l : ld (hl),b
res 3,h
;*** line 40/64 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#44
inc l : ld (hl),#CC
inc l : ld (hl),#CC
inc l : ld (hl),#8D
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/64 ***
ld (hl),#20
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#20
dec l : ld (hl),#41
set 3,h
;*** line 42/64 ***
ld (hl),#5
inc l : ld (hl),#A
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),#88
set 4,h
;*** line 43/64 ***
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#20
dec l : ld (hl),#10
res 3,h
;*** line 44/64 ***
ld (hl),#5
inc l : ld (hl),#A
inc l : ld (hl),d
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 45/64 ***
dec l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#44
dec l : ld (hl),#88
dec l : ld (hl),#44
set 3,h
;*** line 46/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),#44
inc l : ld a,(hl) : and #55 : ld (hl),a
res 4,h
;*** line 47/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 48/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#88
dec l : ld (hl),#44
set 3,h
;*** line 50/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),#CC
inc l : ld a,(hl) : and #55 : ld (hl),a
set 4,h
;*** line 51/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 52/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),#CC
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 53/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#25
dec l : ld (hl),b
dec l : ld (hl),#44
set 3,h
;*** line 54/64 ***
ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
res 4,h
;*** line 55/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),#25
dec l : ld (hl),#88
dec l : ld (hl),#44
res 3,h
;*** line 56/64 ***
ld (hl),#44
inc l : ld (hl),#88
inc l : ld (hl),#25
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),d
set 3,h
;*** line 58/64 ***
ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
set 4,h
;*** line 59/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 60/64 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 61/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 62/64 ***
ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
res 4,h
;*** line 63/64 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),d
res 3,h
;*** line 64/64 ***
ld (hl),d
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
ret
; #DBG curx=5 flux=346 on 512