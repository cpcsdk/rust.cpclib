ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#C0
ld de,#5AA
ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 3,h
;*** line 4/16 ***
inc l : ld a,(hl) : and #55 : ld (hl),a
set 5,h
;*** line 5/16 ***
inc l : inc l : inc l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#10
dec l : ld (hl),b
set 3,h
;*** line 6/16 ***
ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld (hl),#20
inc l : ld (hl),b
inc l : ld (hl),#80
res 4,h
;*** line 7/16 ***
dec l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),c
dec l : ld (hl),#40
res 3,h
;*** line 8/16 ***
ld (hl),#50
inc l : ld (hl),c
inc l : ld a,(hl) : and #55 : ld (hl),a
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
inc l : inc l : inc l : ld a,(hl) : and #55 : ld (hl),a
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld (hl),#4A
dec l : ld (hl),d
set 3,h
;*** line 10/16 ***
ld (hl),d
inc l : ld (hl),#5A
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld a,(hl) : and #55 : ld (hl),a
set 4,h
;*** line 11/16 ***
ld (hl),b
dec l : ld (hl),#80
dec l : ld (hl),c
dec l : ld (hl),#1E
dec l : ld (hl),d
res 3,h
;*** line 12/16 ***
ld (hl),d
inc l : ld (hl),#1A
inc l : ld (hl),#80
inc l : ld (hl),b
inc l : ld (hl),b
set 5,h
;*** line 13/16 ***
ld (hl),b
dec l : ld (hl),c
dec l : ld (hl),c
dec l : ld (hl),#4E
dec l : ld (hl),d
set 3,h
;*** line 14/16 ***
ld (hl),d
inc l : ld (hl),#4A
inc l : ld (hl),c
inc l : ld (hl),b
inc l : ld (hl),b
res 4,h
;*** line 15/16 ***
ld (hl),b
dec l : ld (hl),#C4
dec l : ld (hl),c
dec l : ld (hl),#5A
dec l : ld (hl),#44
res 3,h
;*** line 16/16 ***
ld (hl),#14
inc l : ld (hl),#1E
inc l : ld (hl),c
inc l : ld (hl),c
inc l : ld (hl),b
ret
; #DBG curx=7 flux=69 on 176
