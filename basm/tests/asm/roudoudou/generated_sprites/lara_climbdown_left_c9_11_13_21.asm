ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#AA
ld de,#5580
inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 60/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
set 5,h
;*** line 61/72 ***
inc l : inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#5
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#40
set 3,h
;*** line 62/72 ***
ld (hl),#40
inc l : ld (hl),#10
inc l : ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 63/72 ***
dec l : ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),#50
res 3,h
;*** line 64/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
inc l : ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld (hl),#1A
dec l : ld (hl),#5
dec l : ld (hl),#87
dec l : ld (hl),#40
set 3,h
;*** line 66/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#25
inc l : ld (hl),#38
inc l : ld (hl),#C0
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 67/72 ***
ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),#A
dec l : ld (hl),#F
dec l : ld (hl),#14
res 3,h
;*** line 68/72 ***
ld (hl),#5
inc l : ld (hl),#F
inc l : ld (hl),#4E
inc l : ld (hl),#C0
inc l : ld (hl),e
inc l : ld a,(hl) : and d : ld (hl),a
set 5,h
;*** line 69/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),e
dec l : ld (hl),b
dec l : ld (hl),#A0
dec l : ld (hl),#25
dec l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#85
inc l : ld (hl),#88
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and d : ld (hl),a
res 4,h
;*** line 71/72 ***
ld a,(hl) : and d : ld (hl),a
dec l : ld (hl),#C4
dec l : ld (hl),b
dec l : ld (hl),#A
dec l : ld (hl),#F
dec l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 72/72 ***
ld (hl),#10
inc l : ld (hl),#F
inc l : ld (hl),#4A
inc l : ld (hl),#C0
inc l : ld (hl),#C0
inc l : ld (hl),b
ret
; #DBG curx=10 flux=98 on 792
