ex hl,de
;*** line 1/72 ***
ld a,l : add 192 : ld l,a : ld a,h : adc #19 : ld h,a
;*** line 59/72 ***
ld bc,#A
ld de,#5580
ld a,l : add 5 : ld l,a : ld a,(hl) : and d : ld (hl),a
res 3,h
set 5,h
;*** line 61/72 ***
dec l : dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#25
inc l : ld (hl),#A0
set 3,h
;*** line 62/72 ***
ld (hl),b
dec l : ld (hl),#1A
dec l : ld (hl),b
dec l : ld (hl),b
res 4,h
;*** line 63/72 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#A0
res 3,h
;*** line 64/72 ***
ld (hl),e
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 65/72 ***
dec l : dec l : dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),c
inc l : ld a,(hl) : and d : ld (hl),a
set 3,h
;*** line 66/72 ***
dec l : ld (hl),c
dec l : ld (hl),e
dec l : ld (hl),e
dec l : ld (hl),b
set 4,h
;*** line 67/72 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#10
inc l : ld (hl),c
res 3,h
;*** line 68/72 ***
ld (hl),c
dec l : ld (hl),#44
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 69/72 ***
ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#C0
inc l : ld (hl),#E5
inc l : ld (hl),#A0
set 3,h
;*** line 70/72 ***
ld (hl),#20
dec l : ld (hl),#F
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),b
res 4,h
;*** line 71/72 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#F
inc l : ld (hl),c
res 3,h
;*** line 72/72 ***
ld (hl),c
dec l : ld (hl),#1A
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld (hl),b
ret
; #DBG curx=1 flux=58 on 720
