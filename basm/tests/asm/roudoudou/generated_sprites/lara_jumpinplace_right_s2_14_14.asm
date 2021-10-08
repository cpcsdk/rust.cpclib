ex hl,de
;*** line 1/72 ***
ld bc,#55
ld de,#C080
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),d
inc l : ld (hl),#10
inc l : ld (hl),#20
set 3,h
;*** line 2/72 ***
ld (hl),#28
dec l : ld (hl),d
dec l : ld (hl),d
dec l : ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 3/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#1A
res 3,h
;*** line 4/72 ***
ld (hl),#20
dec l : ld (hl),d
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 5/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#D2
inc l : ld (hl),#20
set 3,h
;*** line 6/72 ***
ld (hl),b
dec l : ld (hl),#F
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 7/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#B4
inc l : ld (hl),#A
res 3,h
;*** line 8/72 ***
ld (hl),#A
dec l : ld (hl),#2D
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#F
inc l : ld (hl),#20
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 10/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#70
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 11/72 ***
inc l : ld (hl),#40
inc l : ld (hl),b
inc l : ld (hl),#F0
inc l : ld (hl),#FC
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 12/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#7
dec l : ld (hl),#F0
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 13/72 ***
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#54
inc l : ld (hl),#AC
set 3,h
;*** line 14/72 ***
ld (hl),#A
dec l : ld (hl),#54
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 15/72 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#AC
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 16/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C
dec l : ld (hl),#D0
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 17/72 ***
inc l : inc l : ld (hl),#54
inc l : ld (hl),#A
set 3,h
;*** line 18/72 ***
ld (hl),#A
dec l : ld (hl),#54
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 19/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#E8
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 20/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A8
dec l : ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 21/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),e
set 3,h
;*** line 22/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),b
res 4,h
;*** line 23/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),#88
res 3,h
;*** line 24/72 ***
ld (hl),#AC
dec l : ld (hl),#98
dec l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 25/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),b
set 3,h
;*** line 26/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 27/72 ***
ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),b
res 3,h
;*** line 28/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),b
set 5,h
;*** line 29/72 ***
ld (hl),#40
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 30/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#40
res 4,h
;*** line 31/72 ***
ld (hl),#40
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : or #20 :ld (hl),a
res 3,h
;*** line 32/72 ***
ld (hl),#20
dec l : ld (hl),#CC
dec l : ld (hl),#40
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 33/72 ***
ld (hl),#44
inc l : ld (hl),#C8
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 34/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#44
set 4,h
;*** line 35/72 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),e
res 3,h
;*** line 36/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#44
set 5,h
;*** line 37/72 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#CC
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 38/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#1A
dec l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 39/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),b
res 3,h
;*** line 40/72 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld a,(hl) : and #AA : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#1A
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 42/72 ***
dec l : ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 43/72 ***
ld (hl),b
inc l : ld (hl),e
res 3,h
;*** line 44/72 ***
ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 45/72 ***
ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 46/72 ***
ld (hl),e
dec l : ld (hl),b
res 4,h
;*** line 47/72 ***
ld (hl),b
inc l : ld (hl),e
res 3,h
;*** line 48/72 ***
ld (hl),e
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld (hl),b
inc l : ld (hl),e
set 3,h
;*** line 50/72 ***
ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 51/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
res 3,h
;*** line 52/72 ***
ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
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
; #DBG curx=4 flux=220 on 792
