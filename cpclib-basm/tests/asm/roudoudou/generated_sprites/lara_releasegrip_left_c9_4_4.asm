ex hl,de
;*** line 1/16 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #8 : ld h,a
;*** line 10/16 ***
ld bc,#AA
ld d,64
dec l : ld a,(hl) : and c : ld (hl),a
set 4,h
;*** line 11/16 ***
ld (hl),#50
inc l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
;*** line 12/16 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
set 5,h
;*** line 13/16 ***
ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
set 3,h
;*** line 14/16 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
res 4,h
;*** line 15/16 ***
ld (hl),d
inc l : ld a,(hl) : and #55 : ld (hl),a
res 3,h
;*** line 16/16 ***
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),d
ret
; #DBG curx=4 flux=20 on 176
