ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#55
ld d,10
inc l : inc l : inc l : inc l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
set 5,h
;*** line 5/16 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#25
inc l : ld (hl),#80
set 3,h
;*** line 6/16 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#1A
dec l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 7/16 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#80
res 3,h
;*** line 8/16 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#44
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),d
set 3,h
;*** line 10/16 ***
ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 4,h
;*** line 11/16 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),d
res 3,h
;*** line 12/16 ***
ld (hl),d
dec l : ld (hl),#10
dec l : ld (hl),b
dec l : ld (hl),b
set 5,h
;*** line 13/16 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),#F
inc l : ld (hl),#82
set 3,h
;*** line 14/16 ***
ld (hl),#A0
dec l : ld (hl),#2D
dec l : ld (hl),#C8
dec l : ld (hl),#40
dec l : ld (hl),b
res 4,h
;*** line 15/16 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#85
inc l : ld (hl),d
res 3,h
;*** line 16/16 ***
ld (hl),d
dec l : ld (hl),#5
dec l : ld (hl),#40
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
ret
; #DBG curx=2 flux=56 on 176
