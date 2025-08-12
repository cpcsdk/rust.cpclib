ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#AA
ld de,#800F
dec l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
set 5,h
;*** line 61/72 ***
dec l : ld (hl),#40
inc l : ld (hl),d
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld a,(hl) : and #55 : ld (hl),a
set 3,h
;*** line 62/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#50
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#40
res 4,h
;*** line 63/72 ***
ld (hl),#44
inc l : ld (hl),d
inc l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
;*** line 64/72 ***
dec l : ld (hl),b
dec l : ld (hl),b
dec l : dec l : ld (hl),b
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld (hl),b
inc l : ld (hl),#25
inc l : ld (hl),b
inc l : ld (hl),#F0
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 66/72 ***
ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#4A
dec l : ld (hl),#10
dec l : ld (hl),#25
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 67/72 ***
inc l : ld (hl),#45
inc l : ld (hl),#30
inc l : ld (hl),#8A
inc l : ld (hl),#C0
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C8
dec l : ld (hl),#A0
dec l : ld (hl),e
dec l : ld (hl),#5
set 5,h
;*** line 69/72 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#88
inc l : ld (hl),#40
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
set 3,h
;*** line 70/72 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),d
dec l : ld (hl),d
dec l : ld (hl),#88
dec l : ld (hl),e
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 71/72 ***
ld (hl),#10
inc l : ld (hl),e
inc l : ld (hl),#A0
inc l : ld (hl),#C0
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
;*** line 72/72 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#C4
dec l : ld (hl),#8A
dec l : ld (hl),e
dec l : ld (hl),#5
ret
; #DBG curx=4 flux=85 on 720
