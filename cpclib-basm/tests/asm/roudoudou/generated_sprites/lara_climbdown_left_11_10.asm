ex hl,de
;*** line 1/144 ***
ld a,h : add 1 : ld h,a
;*** line 33/144 ***
ld bc,#AA
ld de,#5540
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 34/144 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#80
set 4,h
;*** line 35/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 36/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 37/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
set 3,h
;*** line 38/144 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 39/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 40/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 41/144 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),e
dec l : ld (hl),b
set 3,h
;*** line 42/144 ***
ld (hl),b
inc l : ld (hl),#44
inc l : ld (hl),#C8
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 4,h
;*** line 43/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#68
dec l : ld (hl),#C4
dec l : ld (hl),#E0
dec l : ld (hl),b
res 3,h
;*** line 44/144 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 45/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#F
dec l : ld (hl),#F
dec l : ld (hl),#2D
dec l : ld (hl),#10
dec l : ld (hl),b
set 3,h
;*** line 46/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#F
inc l : ld (hl),#F
inc l : ld (hl),#5E
inc l : ld (hl),#8
res 4,h
;*** line 47/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#2D
dec l : ld (hl),#5A
dec l : ld (hl),#25
dec l : ld (hl),#5
dec l : ld (hl),b
res 3,h
;*** line 48/144 ***
ld (hl),e
inc l : ld (hl),#A5
inc l : ld (hl),#F
inc l : ld (hl),#5A
inc l : ld (hl),#80
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 49/144 ***
ld (hl),#A8
dec l : ld (hl),#D6
dec l : ld (hl),#4B
dec l : ld (hl),#A5
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 50/144 ***
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#C1
inc l : ld (hl),#B0
inc l : ld (hl),#A8
set 4,h
;*** line 51/144 ***
ld (hl),#A8
dec l : ld (hl),#DC
dec l : ld (hl),#85
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 52/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#85
inc l : ld (hl),#B0
inc l : ld (hl),#A8
set 5,h
;*** line 53/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#B9
dec l : ld (hl),#85
dec l : ld (hl),#1E
dec l : ld (hl),b
set 3,h
;*** line 54/144 ***
ld (hl),b
inc l : ld (hl),#1A
inc l : ld (hl),#64
inc l : ld (hl),#88
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#AC
dec l : ld (hl),#85
dec l : ld (hl),#1A
dec l : ld (hl),#54
res 3,h
;*** line 56/144 ***
ld (hl),b
inc l : ld (hl),#3C
inc l : ld (hl),#85
inc l : ld (hl),#A8
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/144 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#98
dec l : ld (hl),#C6
dec l : ld (hl),#30
dec l : ld (hl),#10
set 3,h
;*** line 58/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#8D
inc l : ld (hl),#C6
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 59/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C8
dec l : ld (hl),#4E
dec l : ld (hl),#98
dec l : ld (hl),#CC
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 60/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C4
inc l : ld (hl),#98
inc l : ld (hl),#64
inc l : ld (hl),#CC
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 61/144 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),#CC
dec l : ld (hl),#C0
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 62/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#C0
inc l : ld (hl),b
res 4,h
;*** line 63/144 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),e
dec l : ld (hl),#C8
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 64/144 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#CC
inc l : ld (hl),#CC
inc l : ld (hl),#E8
inc l : ld (hl),#80
inc l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/144 ***
ld (hl),#80
dec l : ld (hl),b
dec l : ld (hl),b
dec l : dec l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/144 ***
inc l : inc l : inc l : ld (hl),b
inc l : ld (hl),b
set 4,h
;*** line 67/144 ***
ld (hl),b
res 3,h
;*** line 68/144 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
;*** line 69/144 ***
;*** line 70/144 ***
;*** line 71/144 ***
;*** line 72/144 ***
;*** line 73/144 ***
;*** line 74/144 ***
;*** line 75/144 ***
;*** line 76/144 ***
;*** line 77/144 ***
;*** line 78/144 ***
;*** line 79/144 ***
;*** line 80/144 ***
;*** line 81/144 ***
;*** line 82/144 ***
;*** line 83/144 ***
;*** line 84/144 ***
;*** line 85/144 ***
;*** line 86/144 ***
;*** line 87/144 ***
;*** line 88/144 ***
;*** line 89/144 ***
;*** line 90/144 ***
;*** line 91/144 ***
;*** line 92/144 ***
;*** line 93/144 ***
;*** line 94/144 ***
;*** line 95/144 ***
;*** line 96/144 ***
;*** line 97/144 ***
;*** line 98/144 ***
;*** line 99/144 ***
;*** line 100/144 ***
;*** line 101/144 ***
;*** line 102/144 ***
;*** line 103/144 ***
;*** line 104/144 ***
;*** line 105/144 ***
;*** line 106/144 ***
;*** line 107/144 ***
;*** line 108/144 ***
;*** line 109/144 ***
;*** line 110/144 ***
;*** line 111/144 ***
;*** line 112/144 ***
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
; #DBG curx=8 flux=224 on 1584