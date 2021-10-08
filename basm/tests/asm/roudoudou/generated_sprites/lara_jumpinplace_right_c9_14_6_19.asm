ex hl,de
;*** line 1/16 ***
ld a,h : add 48 : ld h,a
;*** line 5/16 ***
ld bc,#F
ld de,#2055
ld a,l : add 5 : ld l,a : ld (hl),#80
dec l : ld (hl),c
dec l : ld a,(hl) : and #AA : ld (hl),a
set 3,h
;*** line 6/16 ***
dec l : dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#4A
inc l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 7/16 ***
ld (hl),#88
dec l : ld (hl),#85
dec l : ld a,(hl) : and #AA : ld (hl),a
res 3,h
;*** line 8/16 ***
inc l : ld (hl),b
inc l : ld (hl),b
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
dec l : ld (hl),#A
dec l : ld (hl),b
dec l : ld (hl),#40
dec l : ld (hl),b
set 3,h
;*** line 10/16 ***
dec l : ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),d
set 4,h
;*** line 11/16 ***
ld (hl),d
dec l : ld (hl),c
dec l : ld (hl),#14
dec l : ld (hl),b
dec l : ld (hl),b
res 3,h
;*** line 12/16 ***
ld a,(hl) : and #AA : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),#50
inc l : ld (hl),#5
inc l : ld (hl),d
set 5,h
;*** line 13/16 ***
ld (hl),#A0
dec l : ld (hl),c
dec l : ld (hl),#C0
dec l : ld (hl),#C0
dec l : ld (hl),b
set 3,h
;*** line 14/16 ***
ld (hl),b
inc l : ld (hl),#C8
inc l : ld (hl),#C0
inc l : ld (hl),c
inc l : ld (hl),d
res 4,h
;*** line 15/16 ***
ld (hl),#88
dec l : ld (hl),#25
dec l : ld (hl),#C8
dec l : ld (hl),#40
dec l : ld (hl),b
res 3,h
;*** line 16/16 ***
ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#C0
inc l : ld (hl),c
inc l : ld (hl),d
ret
; #DBG curx=7 flux=58 on 176
