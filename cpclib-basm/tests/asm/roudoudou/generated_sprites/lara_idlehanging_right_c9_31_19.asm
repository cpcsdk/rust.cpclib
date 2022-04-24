ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#55
ld de,#A0F
ld a,l : add 5 : ld l,a : ld a,(hl) : and c : ld (hl),a
res 3,h
set 5,h
;*** line 5/16 ***
dec l : dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),e
inc l : ld (hl),#A0
set 3,h
;*** line 6/16 ***
ld (hl),b
dec l : ld (hl),#1A
dec l : ld a,(hl) : and #AA : ld (hl),a
dec l : ld a,(hl) : and #AA : ld (hl),a
res 4,h
;*** line 7/16 ***
inc l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),#5
inc l : ld (hl),#28
res 3,h
;*** line 8/16 ***
ld (hl),#80
dec l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
dec l : dec l : dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 10/16 ***
dec l : ld (hl),d
dec l : ld (hl),#10
dec l : ld (hl),#80
dec l : ld (hl),b
set 4,h
;*** line 11/16 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#10
inc l : ld (hl),d
res 3,h
;*** line 12/16 ***
ld (hl),d
dec l : ld (hl),#10
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and #AA : ld (hl),a
set 5,h
;*** line 13/16 ***
ld (hl),b
inc l : ld (hl),#40
inc l : ld (hl),#C8
inc l : ld (hl),#69
inc l : ld (hl),#88
set 3,h
;*** line 14/16 ***
ld (hl),#20
dec l : ld (hl),e
dec l : ld (hl),#C0
dec l : ld (hl),#C4
dec l : ld (hl),b
res 4,h
;*** line 15/16 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#CC
inc l : ld (hl),e
inc l : ld (hl),d
res 3,h
;*** line 16/16 ***
ld (hl),d
dec l : ld (hl),e
dec l : ld (hl),#44
dec l : ld (hl),b
dec l : ld (hl),b
ret
; #DBG curx=0 flux=61 on 112
