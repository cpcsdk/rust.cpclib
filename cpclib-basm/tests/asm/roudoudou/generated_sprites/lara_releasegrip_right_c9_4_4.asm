ex hl,de
;*** line 1/16 ***
ld a,l : add 64 : ld l,a : ld a,h : adc #8 : ld h,a
;*** line 10/16 ***
ld bc,#A
ld d,85
inc l : inc l : inc l : inc l : ld a,(hl) : and d : ld (hl),a
set 4,h
;*** line 11/16 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#88
res 3,h
;*** line 12/16 ***
ld (hl),#88
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 13/16 ***
ld (hl),#10
inc l : ld (hl),c
set 3,h
;*** line 14/16 ***
ld (hl),c
dec l : ld (hl),#5
res 4,h
;*** line 15/16 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),c
res 3,h
;*** line 16/16 ***
ld (hl),c
dec l : ld a,(hl) : and #AA : ld (hl),a
ret
; #DBG curx=5 flux=18 on 176
