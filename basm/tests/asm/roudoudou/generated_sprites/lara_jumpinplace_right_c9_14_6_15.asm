ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#C0
ld de,#A55
ld a,l : add 5 : ld l,a : ld a,(hl) : and e : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 4/16 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 5/16 ***
inc l : ld (hl),b
dec l : ld (hl),d
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
set 3,h
;*** line 6/16 ***
ld (hl),#40
inc l : ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),d
inc l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 7/16 ***
ld (hl),#80
dec l : ld (hl),#F
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 8/16 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#F
inc l : ld (hl),#A0
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
dec l : ld (hl),d
dec l : ld (hl),#90
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 10/16 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#A5
inc l : ld (hl),d
set 4,h
;*** line 11/16 ***
ld (hl),d
dec l : ld (hl),#2D
dec l : ld (hl),c
dec l : ld (hl),#40
dec l : ld (hl),b
res 3,h
;*** line 12/16 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#25
inc l : ld (hl),d
set 5,h
;*** line 13/16 ***
ld (hl),d
dec l : ld (hl),#8D
dec l : ld (hl),c
dec l : ld (hl),c
dec l : ld (hl),b
set 3,h
;*** line 14/16 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),c
inc l : ld (hl),c
inc l : ld (hl),#20
res 4,h
;*** line 15/16 ***
ld (hl),#88
dec l : ld (hl),#A5
dec l : ld (hl),c
dec l : ld (hl),#C8
dec l : ld (hl),b
res 3,h
;*** line 16/16 ***
ld (hl),b
inc l : ld (hl),c
inc l : ld (hl),c
inc l : ld (hl),#2D
inc l : ld (hl),#28
ret
; #DBG curx=7 flux=68 on 176
