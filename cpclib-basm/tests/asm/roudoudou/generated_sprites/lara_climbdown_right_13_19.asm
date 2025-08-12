ex hl,de
;*** line 1/144 ***
ld a,l : add 128 : ld l,a : ld a,h : adc #9 : ld h,a
;*** line 50/144 ***
ld bc,#AA
ld de,#5580
inc l : inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
set 4,h
;*** line 51/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 52/144 ***
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 53/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#C4
inc l : ld (hl),#40
inc l : ld (hl),b
set 3,h
;*** line 54/144 ***
ld (hl),b
dec l : ld (hl),#D0
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 55/144 ***
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 56/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/144 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#C0
inc l : ld (hl),#94
inc l : ld (hl),b
set 3,h
;*** line 58/144 ***
ld (hl),b
dec l : ld (hl),#A5
dec l : ld (hl),#C0
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),#40
set 4,h
;*** line 59/144 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#30
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 60/144 ***
ld (hl),b
dec l : ld (hl),#B0
dec l : ld (hl),#C0
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 61/144 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#A5
inc l : ld (hl),#1A
inc l : ld (hl),#4E
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 62/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#92
dec l : ld (hl),#5
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
res 4,h
;*** line 63/144 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#8D
inc l : ld (hl),#F
inc l : ld (hl),#68
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 64/144 ***
dec l : ld (hl),b
dec l : ld (hl),#1A
dec l : ld (hl),#41
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#2D
inc l : ld (hl),#85
inc l : ld (hl),#C9
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#40
dec l : ld (hl),#90
dec l : ld (hl),#F
dec l : ld (hl),#14
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#C1
inc l : ld (hl),#92
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 68/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C2
dec l : ld (hl),#5
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 69/144 ***
inc l : inc l : ld (hl),#44
inc l : ld (hl),#FC
inc l : ld a,(hl) : and d : or #8 :ld (hl),a
set 3,h
;*** line 70/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AC
dec l : ld (hl),#54
res 4,h
;*** line 71/144 ***
ld (hl),#D0
inc l : ld (hl),#74
inc l : ld (hl),#20
res 3,h
;*** line 72/144 ***
ld a,(hl) : and d : or #20 :ld (hl),a
dec l : ld (hl),#25
dec l : ld (hl),#50
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 73/144 ***
inc l : ld (hl),#54
inc l : ld (hl),#AC
set 3,h
;*** line 74/144 ***
ld (hl),#1A
dec l : ld (hl),#54
set 4,h
;*** line 75/144 ***
ld (hl),#54
inc l : ld (hl),#AD
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 76/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AD
dec l : ld (hl),#54
set 5,h
;*** line 77/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 78/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#FC
dec l : ld (hl),#10
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 79/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#33
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 80/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AD
dec l : ld (hl),#54
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 81/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 82/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#CC
dec l : ld (hl),#40
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 83/144 ***
inc l : ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),b
res 3,h
;*** line 84/144 ***
ld (hl),b
dec l : ld (hl),#CC
dec l : ld (hl),#40
set 5,h
;*** line 85/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#88
set 3,h
;*** line 86/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#88
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 87/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#88
res 3,h
;*** line 88/144 ***
ld (hl),#88
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 89/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 90/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 91/144 ***
ld (hl),b
inc l : ld (hl),#C4
inc l : ld (hl),#C0
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 92/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C0
dec l : ld (hl),#44
dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
;*** line 93/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),#88
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 94/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#64
dec l : ld (hl),b
res 4,h
;*** line 95/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#44
inc l : ld (hl),#CC
inc l : ld (hl),#C0
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 96/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C0
dec l : ld (hl),#CC
dec l : ld (hl),#40
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 97/144 ***
dec l : ld (hl),#10
inc l : ld (hl),#4E
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 98/144 ***
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#A
dec l : ld (hl),b
set 4,h
;*** line 99/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),e
res 3,h
;*** line 100/144 ***
ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#40
set 5,h
;*** line 101/144 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
set 3,h
;*** line 102/144 ***
ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 103/144 ***
ld (hl),b
inc l : ld (hl),e
inc l : inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
res 3,h
;*** line 104/144 ***
ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 105/144 ***
ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
inc l : inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
set 3,h
;*** line 106/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
set 4,h
;*** line 107/144 ***
inc l : inc l : inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 108/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : dec l : dec l : ld a,(hl) : and c : ld (hl),a
set 5,h
set 3,h
res 4,h
;*** line 111/144 ***
inc l : inc l : inc l : inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 112/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
;*** line 113/144 ***
;*** line 114/144 ***
;*** line 115/144 ***
;*** line 116/144 ***
;*** line 117/144 ***
;*** line 118/144 ***
;*** line 119/144 ***
;*** line 120/144 ***
;*** line 121/144 ***
;*** line 122/144 ***
;*** line 123/144 ***
;*** line 124/144 ***
;*** line 125/144 ***
;*** line 126/144 ***
;*** line 127/144 ***
;*** line 128/144 ***
;*** line 129/144 ***
;*** line 130/144 ***
;*** line 131/144 ***
;*** line 132/144 ***
;*** line 133/144 ***
;*** line 134/144 ***
;*** line 135/144 ***
;*** line 136/144 ***
;*** line 137/144 ***
;*** line 138/144 ***
;*** line 139/144 ***
;*** line 140/144 ***
;*** line 141/144 ***
;*** line 142/144 ***
;*** line 143/144 ***
;*** line 144/144 ***
ret
; #DBG curx=4 flux=343 on 1584
