ex hl,de
;*** line 1/72 ***
ld a,l : add 128 : ld l,a : ld a,h : adc #9 : ld h,a
;*** line 50/72 ***
ld bc,#55
ld de,#AA80
inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 51/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 52/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
set 5,h
;*** line 53/72 ***
dec l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#C8
inc l : ld (hl),#C8
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 54/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),#E0
dec l : ld (hl),b
res 4,h
;*** line 55/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#C4
inc l : ld (hl),b
res 3,h
;*** line 56/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 57/72 ***
ld (hl),b
inc l : ld (hl),#68
inc l : ld (hl),#C0
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 58/72 ***
ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),#5A
dec l : ld (hl),b
set 4,h
;*** line 59/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#34
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 60/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#5A
dec l : ld (hl),b
set 5,h
;*** line 61/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#25
inc l : ld (hl),#5A
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 62/72 ***
ld (hl),b
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld (hl),#70
dec l : ld (hl),#F
dec l : ld (hl),#50
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 63/72 ***
ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#C0
inc l : ld (hl),#85
inc l : ld (hl),#4E
inc l : ld (hl),e
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 64/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#82
dec l : ld (hl),#25
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#C6
inc l : ld (hl),#F
inc l : ld (hl),#1E
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 66/72 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#28
dec l : ld (hl),#F
dec l : ld (hl),#30
dec l : ld (hl),e
dec l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 67/72 ***
inc l : inc l : ld a,(hl) : and d : ld (hl),a
inc l : ld (hl),#61
inc l : ld (hl),#C2
inc l : ld a,(hl) : and d : ld (hl),a
res 3,h
;*** line 68/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#A
dec l : ld (hl),#C1
dec l : ld (hl),b
dec l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 69/72 ***
inc l : ld a,(hl) : and d : or #4 :ld (hl),a
inc l : ld (hl),#FC
inc l : ld (hl),#88
set 3,h
;*** line 70/72 ***
ld (hl),#A8
dec l : ld (hl),#5C
dec l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 71/72 ***
ld (hl),#10
inc l : ld (hl),#B8
inc l : ld (hl),#E0
res 3,h
;*** line 72/72 ***
inc l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#A0
dec l : ld (hl),#1A
dec l : ld a,(hl) : and d : or #10 :ld (hl),a
ret
; #DBG curx=5 flux=146 on 792
