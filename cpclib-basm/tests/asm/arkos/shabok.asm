	; Shabok 256 bytes
; Shadebob effect 100% firmware based (slooooww)
; Done with WinAPE assembler (Maxam compatible)
;
; Grim/Arkos^Semilanceata

org &4000
run $

_GRA_SET_PEN EQU &BBDE
_GRA_PLOT_ABSOLUTE EQU &BBEA
_GRA_TEST_ABSOLUTE  EQU &BBF0
_SET_ANGLE_MODE EQU &BD97
_REAL_SINE EQU &BDAC
_REAL_ADDITION EQU &BD7C
_REAL_MOVE EQU &BD61
_REAL_INT EQU &BD73
_REAL_MULTIPLICATION EQU &BD85
_SCR_SET_INK EQU &BC32
_SCR_SET_BORDER  EQU &BC38

; Set the colors
ld bc,0
call _SCR_SET_BORDER

ld hl,shadebob_data_palette
ld a,15
shadebob_setpalette
ld b,(hl)
inc hl
ld c,b
push af
push hl
call _SCR_SET_INK
pop hl
pop af
dec a
jp p,shadebob_setpalette

; Switch to MODE 0 (160x200x16)
ld a,0
call &BC0E

; DEG
; tell the math firmware to deal with angle in degres
ld a,1
call _SET_ANGLE_MODE

; main loop
shadebob_main ; compute X coordinate

; sinus1=INT(amplitude1*SIN(angle1))+320
; angle1=angle1+step1

ld hl,math_var_sinus1
push hl
ld de,math_var_angle1
call _REAL_MOVE
call _REAL_SINE
pop hl
ld de,math_var_amplitude1
call _REAL_MULTIPLICATION
call _REAL_INT

ld hl,320
ld de,(math_var_sinus1)
call shadebob_coordinify
push hl

; angle+=step
ld hl,math_var_angle1
ld de,math_var_angle1_step
call _REAL_ADDITION

; compute Y coordinate

; sinus2=INT(amplitude2*SIN(angle2))+200
; angle2=angle2+step2

ld hl,math_var_sinus2
push hl
ld de,math_var_angle2
call _REAL_MOVE
call _REAL_SINE
pop hl
ld de,math_var_amplitude2
call _REAL_MULTIPLICATION
call _REAL_INT

ld hl,200
ld de,(math_var_sinus2)
call shadebob_coordinify
push hl

; angle+=step
ld hl,math_var_angle2
ld de,math_var_angle2_step
call _REAL_ADDITION

; plot the shadebob
pop hl
pop de

ld ix,shadebob_data_sprite
ld b,9
shadebob_plot_bob_mainloop
push de

ld c,(ix+0)
inc ix
shadebob_plot_bob_pixelloop
sla c
jr z,shadebob_plot_bob_nextline
call c,shadebob_plot_pixel
inc de
inc de
inc de
inc de
jr shadebob_plot_bob_pixelloop
shadebob_plot_bob_nextline call c,shadebob_plot_pixel

pop de
inc hl
inc hl
djnz shadebob_plot_bob_mainloop

jp shadebob_main

math_var_angle1 db 0,0,0,0,0
math_var_angle1_step db 154,153,153,9,132
math_var_amplitude1 db 0,0,0,122,136
math_var_sinus1 db 0,0,0,0,0

math_var_angle2 db 0,0,0,0,0
math_var_angle2_step db 133,235,81,8,131
math_var_amplitude2 db 0,0,0,72,136
math_var_sinus2 db 0,0,0,0,0


shadebob_coordinify:
bit 7,b
jr z,shadebob_coordpos
sbc hl,de
ret
shadebob_coordpos add hl,de
ret

shadebob_plot_pixel:
push bc
push de
push hl
push de
push hl
call _GRA_TEST_ABSOLUTE
inc a
call _GRA_SET_PEN
pop hl
pop de
call _GRA_PLOT_ABSOLUTE
pop hl
pop de
pop bc
ret
shadebob_data_sprite
db %01110000
db %01110000
db %11111000
db %11111000
db %11111000
db %11111000
db %11111000
db %01110000
db %01110000
shadebob_data_palette
db 1,2,5,11,14,20,23,26
db 25,24,16,15,6,4,3,0

