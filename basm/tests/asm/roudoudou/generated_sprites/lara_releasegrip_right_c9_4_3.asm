ex hl,de
;*** line 1/16 ***
set 3,h
set 4,h
;*** line 3/16 ***
ld bc,#55
ld de,#AAA
inc l : inc l : inc l : inc l : ld a,(hl) : and e : ld (hl),a
res 3,h
set 5,h
;*** line 5/16 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),#25
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 6/16 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#1E
dec l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 7/16 ***
inc l : ld (hl),#14
inc l : ld a,(hl) : and c : ld (hl),a
res 3,h
;*** line 8/16 ***
ld a,(hl) : and c : ld (hl),a
dec l : ld (hl),#40
ld a,l : add #40 : ld l,a : ld a,h : adc #E0 : ld h,a
;*** line 9/16 ***
dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),d
inc l : ld a,(hl) : and c : ld (hl),a
set 3,h
;*** line 10/16 ***
dec l : ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
set 4,h
;*** line 11/16 ***
dec l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),d
res 3,h
;*** line 12/16 ***
ld (hl),d
dec l : ld a,(hl) : and e : ld (hl),a
set 5,h
;*** line 13/16 ***
dec l : dec l : dec l : ld a,(hl) : and e : ld (hl),a
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#90
inc l : ld (hl),d
set 3,h
;*** line 14/16 ***
ld (hl),d
dec l : ld (hl),#B0
dec l : ld (hl),b
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
res 4,h
;*** line 15/16 ***
inc l : ld (hl),b
inc l : ld (hl),b
inc l : ld (hl),#10
inc l : ld (hl),d
res 3,h
;*** line 16/16 ***
ld (hl),d
dec l : ld (hl),#10
dec l : ld (hl),b
dec l : ld a,(hl) : and e : ld (hl),a
ret
; #DBG curx=3 flux=53 on 176
