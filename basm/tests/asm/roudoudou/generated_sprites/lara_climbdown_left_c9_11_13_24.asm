ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#5
ld de,#8040
dec l : dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
set 5,h
;*** line 61/72 ***
ld (hl),#44
inc l : ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
set 3,h
;*** line 62/72 ***
inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#10
dec l : ld (hl),b
res 4,h
;*** line 63/72 ***
ld (hl),#14
inc l : ld (hl),d
res 3,h
;*** line 64/72 ***
ld (hl),b
dec l : ld (hl),e
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),c
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 66/72 ***
ld (hl),b
dec l : ld (hl),e
dec l : ld (hl),e
dec l : ld (hl),c
set 4,h
;*** line 67/72 ***
ld (hl),c
inc l : ld (hl),#20
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
;*** line 68/72 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),#20
dec l : ld (hl),c
set 5,h
;*** line 69/72 ***
ld (hl),#50
inc l : ld (hl),#96
inc l : ld (hl),#C0
inc l : ld (hl),d
inc l : ld (hl),b
set 3,h
;*** line 70/72 ***
ld (hl),b
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),#F
dec l : ld (hl),#10
res 4,h
;*** line 71/72 ***
ld (hl),c
inc l : ld (hl),#F
inc l : ld (hl),#C4
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 72/72 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#88
dec l : ld (hl),#25
dec l : ld (hl),c
ret
; #DBG curx=5 flux=56 on 792
