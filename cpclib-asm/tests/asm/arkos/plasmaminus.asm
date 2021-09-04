; Plasmaminus, a 256-bytes intro by Grim/Arkos^Semilanceata
; 20121108

				org &1000

				;;write direct "a:plasmaminus.",_exec
				run _exec

				; 2x8 Dithering levels 
_data_dithering
				db %01000000	; Dark
				db %01000100
				db %01010100
				db %01010101
				
				db %01110101
				db %01110111
				db %01111111
				db %11111111	; Bright
				
				db %01111111
				db %01110111
				db %01011101
				db %01010101
				
				db %00010101
				db %00100010
				db %00001000
				db %00000000	; Dark

				; 256 bytes periodic sin-like wave
_data_wave			equ &1100

_exec
				di
				;*** Setup screen mode & colors ***
				exx
				ld de,&0158 ;46
				out (c),d
				out (c),e
				ld a,&8E
				out (c),a

				;*** Setup CRTC for wide horizontal display ***
				ld de,&0306		; Type 2 Fullscreen fix
				call setCrtcReg
				ld de,&0100+50
				call setCrtcReg
				inc d
				call setCrtcReg

				;*** Precalc 256 bytes exp-based sin-like wave ***
				ld hl,_data_wave
				ld de,_data_wave + 127
				ld b,64
_sinGen
				ld a,b
				exx
				ld hl,0
				ld d,h
				ld e,a
_exp				add hl,de
				dec a
				jr nz,_exp
				add hl,hl
				ld a,h
				exx
				
				ld (hl),a
				ld (de),a
				neg
				add 64
				set 7,l
				set 7,e
				ld (hl),a
				ld (de),a
				res 7,l
				res 7,e
				dec e
				inc l
				
				djnz _sinGen
				
				;*** Generate wobbler pattern ***
				ld iy,_data_wave+64
				ld de,_data_wave
				ld hl,0
				ld  b,_data_dithering / 256
_gfxGen
				ld  a,(de)	;  wave X
				add a,(iy+0)	; +wave Y
				; lookup dithering value
				and %1111
				ld  c,a
				ld  a,(bc)		; Lookup dithering byte
				ld (hl),a		; plot lineAddr.even
				set 3,h
				rrca
				rrca
				rrca
				ld (hl),a		; plot lineAddr.odd
				res 3,h
				inc hl
				; innerloop (X)
				inc e		; next wave X
				inc e
				jr nz,_gfxGen
				inc iyl		; next wave Y
				inc iyl
				; outerloop (Y)
				bit 3,h
				jr z,_gfxGen
				ld a,%00111000
				add a,h
				ld  h,a
				jr nc,_gfxGen
				
				push de
				ld e,c
				push de
				push de
				
				;*** Wobbler clean CRTC transition ***
				call riseVSync
				; R9=1
				ld de,&0901
				call setCrtcReg
				; R4=30
				ld de,&0400+30
				call setCrtcReg
				
				;*** Wobbler display ***
frame
				; wait 128us
				ld b,32 -7-1
				djnz $

				pop bc
				pop de
				pop hl
				inc l
				inc e
				inc e
				dec c
				push hl
				push de
				push bc
				exx
				
				; R4=0
				;ld de,&0400		;3
				;call setCrtcReg	;3
				ld b,&BD			;2
				dw &71ED 			;2 out (c),0
				; R7=1
				ld de,&0701
				call setCrtcReg
				
				ld c,154
wobbler
				; Sum some sinewaves
				exx
				ld a,(bc)
				add a,(hl)
				ex de,hl
				add a,(hl)
				inc e
				inc c
				inc c
				exx
				; A = Wobbler 6-bit line number to display (0-63)

				; Ping-pong pattern (0,127) -> (0,63,0)
				bit 6,a
				jr nz,$+2+1
				cpl

				; Convert wobbler line number into screen offset
				; A = %??aa.bbcc
				ld h,a
				rrca
				rrca
				; A = %cc??.aabb
				ld l,a
				and %11000000
				; A = %cc00.0000 = R13
				ld d,13
				ld e,a
				call setCrtcReg
				
				ld a,h
				and %00110000
				; A = %00aa.0000
				ld h,a
				ld a,l
				and %00000011
				; A = %0000.00bb
				or h
				; A = %aaxx.xxbb = R12
				dec d
				ld e,a
				call setCrtcReg
				
				; Sync to 128us / Waste cycles
				ld b,12
				djnz $
				nop

				; Wobbler display loop
				dec c
				jr nz,wobbler
				
				; End display, prep. VSync
				ld de,&0400+1 ; R4=1
				call setCrtcReg
				; Wait for VSync, 50Hz loop
				call riseVSync
				; Loop forever
				jr frame
				
riseVSync:
				ld b,&F5
				;removed to fit in 256b, CRTC transition might fail
				;in a,(c)
				;rra
				;jr c,riseVSync+2  ; anti-vsync
_waitVSync			in a,(c)
				rra
				jr nc,_waitVSync ; wait-vsync
				ret
setCrtcReg:
				ld b,&BC
				out (c),d
				inc b
				out (c),e
				ret
				
				