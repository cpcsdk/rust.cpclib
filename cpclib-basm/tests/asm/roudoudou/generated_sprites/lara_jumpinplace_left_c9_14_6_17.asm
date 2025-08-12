ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#C0
ld de,#540
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
set 5,h
;*** line 5/16 ***
ld (hl),e
inc l : ld (hl),c
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#80
inc l : ld a,(hl) : and #55 : ld (hl),a
set 3,h
;*** line 6/16 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#20
dec l : ld (hl),d
dec l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 7/16 ***
ld (hl),e
inc l : ld (hl),c
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),b
res 3,h
;*** line 8/16 ***
dec l : ld (hl),b
dec l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),c
dec l : ld (hl),#44
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
inc l : ld (hl),d
inc l : ld (hl),#60
inc l : ld (hl),#A0
inc l : ld (hl),b
inc l : ld (hl),b
set 3,h
;*** line 10/16 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),c
dec l : ld (hl),#F
dec l : ld (hl),d
set 4,h
;*** line 11/16 ***
ld (hl),d
inc l : ld (hl),#1E
inc l : ld (hl),c
inc l : ld (hl),#C8
inc l : ld (hl),b
res 3,h
;*** line 12/16 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),e
dec l : ld (hl),#4B
dec l : ld (hl),d
set 5,h
;*** line 13/16 ***
ld (hl),d
inc l : ld (hl),#F
inc l : ld (hl),b
inc l : ld (hl),e
inc l : ld (hl),#80
set 3,h
;*** line 14/16 ***
ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#4B
dec l : ld (hl),#10
res 4,h
;*** line 15/16 ***
ld (hl),#50
inc l : ld (hl),#87
inc l : ld (hl),e
inc l : ld (hl),c
inc l : ld (hl),#88
res 3,h
;*** line 16/16 ***
ld (hl),#80
dec l : ld (hl),#CC
dec l : ld (hl),c
dec l : ld (hl),#4B
dec l : ld (hl),#14
ret
; #DBG curx=3 flux=70 on 176
