ex hl,de
;*** line 1/72 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #1 : ld h,a
;*** line 41/72 ***
ld bc,#55
ld de,#8040
ld a,l : add 5 : ld l,a : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 42/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),b
set 4,h
;*** line 43/72 ***
inc l : inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 44/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 45/72 ***
inc l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 46/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#88
inc l : ld (hl),b
res 4,h
;*** line 47/72 ***
ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 48/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),d
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/72 ***
ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 50/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),d
inc l : ld (hl),b
set 4,h
;*** line 51/72 ***
ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),b
res 3,h
;*** line 52/72 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),d
inc l : ld (hl),d
set 5,h
;*** line 53/72 ***
ld (hl),b
dec l : ld (hl),#4A
dec l : ld (hl),#1A
dec l : ld (hl),#87
dec l : ld (hl),#A5
dec l : ld (hl),e
set 3,h
;*** line 54/72 ***
ld (hl),#50
inc l : ld (hl),#E1
inc l : ld (hl),#87
inc l : ld (hl),#F
inc l : ld (hl),#20
inc l : ld (hl),b
res 4,h
;*** line 55/72 ***
ld (hl),b
dec l : ld (hl),#F0
dec l : ld (hl),#4B
dec l : ld (hl),#F
dec l : ld (hl),#A5
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 56/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#85
inc l : ld (hl),#A5
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#A
dec l : ld (hl),#1A
dec l : ld (hl),#F4
dec l : ld (hl),#14
set 3,h
;*** line 58/72 ***
ld (hl),#54
inc l : ld (hl),#AD
inc l : ld (hl),#C0
inc l : ld (hl),#A
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 59/72 ***
dec l : dec l : ld (hl),#20
dec l : ld (hl),#10
dec l : ld (hl),#E
dec l : ld (hl),#54
res 3,h
;*** line 60/72 ***
ld (hl),#54
inc l : ld (hl),#FC
inc l : ld (hl),#90
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 61/72 ***
ld (hl),d
dec l : ld (hl),#8D
dec l : ld (hl),#CC
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 62/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#60
inc l : ld (hl),#CC
inc l : ld (hl),#10
inc l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#9C
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 64/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#88
inc l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),#E9
dec l : ld (hl),#88
dec l : ld (hl),b
set 3,h
;*** line 66/72 ***
ld (hl),b
inc l : ld (hl),#88
inc l : ld (hl),#F
inc l : ld (hl),b
set 4,h
;*** line 67/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),d
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),#2D
inc l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 69/72 ***
ld (hl),#20
dec l : ld (hl),#FC
dec l : ld (hl),d
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#10
inc l : ld (hl),#FC
inc l : ld (hl),b
res 4,h
;*** line 71/72 ***
ld (hl),b
dec l : ld (hl),#76
dec l : ld (hl),e
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#D4
inc l : ld a,(hl) : and c : ld (hl),a
ret
; #DBG curx=5 flux=195 on 792
