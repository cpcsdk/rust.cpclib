ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#AA
ld de,#555
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
set 5,h
;*** line 5/16 ***
dec l : ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#C0
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 6/16 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),#C0
dec l : ld a,(hl) : and c : ld (hl),a
res 4,h
;*** line 7/16 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#A0
res 3,h
;*** line 8/16 ***
ld (hl),b
dec l : ld a,(hl) : and c : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
ld a,(hl) : and c : ld (hl),a
inc l : ld (hl),#40
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 10/16 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),d
set 4,h
;*** line 11/16 ***
ld (hl),d
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 12/16 ***
dec l : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),d
set 5,h
;*** line 13/16 ***
ld (hl),d
inc l : ld (hl),#20
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and e : ld (hl),a
set 3,h
;*** line 14/16 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),#70
dec l : ld (hl),d
res 4,h
;*** line 15/16 ***
ld (hl),d
inc l : ld (hl),#20
inc l : ld (hl),#80
inc l : ld (hl),b
res 3,h
;*** line 16/16 ***
ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),#20
dec l : ld (hl),d
ret
; #DBG curx=4 flux=54 on 176
