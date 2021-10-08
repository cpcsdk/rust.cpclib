ex hl,de
;*** line 1/72 ***
ld a,l : add 128 : ld l,a : ld a,h : adc #31 : ld h,a
;*** line 53/72 ***
ld bc,#40
ld de,#55AA
inc l : inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 54/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),c
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 55/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
ld a,l : add 64 : ld l,a : ld a,h : adc #D8 : ld h,a
;*** line 57/72 ***
dec l : dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 58/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#50
dec l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 59/72 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#34
inc l : ld (hl),#C4
inc l : ld (hl),#88
inc l : ld (hl),#80
res 3,h
;*** line 60/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#50
dec l : ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 61/72 ***
dec l : ld (hl),c
inc l : ld (hl),#5
inc l : ld (hl),#4B
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),#80
set 3,h
;*** line 62/72 ***
ld (hl),b
dec l : ld (hl),c
dec l : ld (hl),#C0
dec l : ld (hl),#F
dec l : ld (hl),#3C
dec l : ld (hl),c
res 4,h
;*** line 63/72 ***
ld (hl),c
inc l : ld (hl),#80
inc l : ld (hl),#3C
inc l : ld (hl),c
inc l : ld (hl),#C4
inc l : ld (hl),#80
res 3,h
;*** line 64/72 ***
ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),#C0
dec l : ld (hl),#34
dec l : ld (hl),b
dec l : ld (hl),c
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld (hl),#44
inc l : ld (hl),#1A
inc l : ld (hl),#F
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),#25
dec l : ld (hl),#87
dec l : ld (hl),c
set 4,h
;*** line 67/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#1A
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),c
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#61
dec l : ld (hl),#25
dec l : ld (hl),b
set 5,h
;*** line 69/72 ***
inc l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#61
inc l : ld (hl),#A
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 70/72 ***
ld (hl),b
dec l : ld (hl),c
dec l : ld (hl),#A
dec l : ld (hl),#1E
dec l : ld (hl),#10
res 4,h
;*** line 71/72 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#A
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#F
dec l : ld a,(hl) : and e : ld (hl),a
ret
; #DBG curx=5 flux=123 on 792
