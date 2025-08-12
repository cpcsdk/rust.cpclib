;    _____         __
;   /  _  \_______|  | ______  ______
;  /  /_\  \_  __ \  |/ /  _ \/  ___/
; /    |    \  | \/    <  <_> )___ \
; \____|__  /__|  |__|_ \____/____  >
;         \/           \/         \/
; Twither, a 256 bytes intro presented at Forever 2009.
; (252 bytes exactly =)
;
; Hires(640x200) and fullscreen twister bar with some sort
; of dithering effet (not exactly what I planned first but
; I miserably failed to squeeze it all in 256 bytes =).
;
; The twister is 256 (mode 2) pixels wide with 256 steps.
;
; It works on CRTC type 0, 1 and 3.
; It should work on CRTC type 4 (not tested)
; It simply can't work on CRTC type 2.
;
; Grim/Arkos^Semilanceata
;
; NOTES
; - content at &0Cxx corrupted by the fx 64us innerloop, DO NOT USE!

;*** Configuration ************************************************************

					; Compile a small build test-code and debug stuff
rubberbar_debug				equ 0

					; Configure CRTC synchronization
					; &E0 - Type 0.
					; &F0 - Type 1, 3 (probably 4).
rubberbar_cnf_crtcsync			equ &F0

					; Sinus lookup tables addresses
rubberbar_data_lut_sinus1024		equ &8800
rubberbar_data_lut_sinus256		equ &0900	; opt - hibyte used as CRTC Select reg 9

;******************************************************************************

					if rubberbar_debug
						org &9F00
						run $
						; clear screen
						ld a,2
						call &BC0E
						jp rubberbar_exec
					else
						run rubberbar_exec
					endif


					org &A000

					; Dithering patterns
rubberbar_data_dither_seq		db 1,17,21,85,87,119,127,255
;rubberbar_data_dither_seq		db 255,127,119,87,85,21,17,1

rubberbar_exec:
					; disable 300Hz interrupts
					di
					; select screen mode 2
					ld bc,&7F8E
					out (c),c
					; set ink 1 color to red (&4C)
					ld de,&014C
					out (c),d
					out (c),e

					; Wild CRTC 6845 initialization
					ld hl,&0701
					;ld de,&0100+16
					ld e,16
					call rubberbar_crtset

;*** Generate a 1024 bytes sinus (8bits unsigned) lookup table *****************

cnf_math_singen_sizeopt_store		equ rubberbar_data_lut_sinus1024
cnf_math_singen_inline			equ 1
cnf_math_singen_unsigned		equ 1
					read "math.singen.sizeopt.asm"
					; Output
					; HL=0
					; DE=0
					; BC=1
					
					; save HL=&0000 (used as VRAM pointer later)
					push hl
					; clear vram
					inc e
					ld (hl),l
					ld bc,&87FF
					ldir
					
;*** Generate a 256 bytes sinus (8bits unsigned) lookup table ******************

					; 256 bytes sinus
					;ld h,rubberbar_data_lut_sinus1024/256
					inc hl
					ld d,rubberbar_data_lut_sinus256/256
					ld c,4
_rubberbar_256b_singen_loop		ld a,(hl)
					add a,e
					ld (de),a
					add hl,bc
					inc e
					jr nz,_rubberbar_256b_singen_loop
					
;*** Generate the twisterbar graphic *******************************************
					
					ld hl,rubberbar_data_lut_sinus1024 + &180 ; pi/2+pi/4
					exx
					pop hl	;ld hl,&0000
					call rubberbar_generator
					call rubberbar_generator
					
;*** 50Hz Twister display loop *************************************************

					; *** 50Hz loop ***
					
rubberbar_sync				ld b,&F5
					in a,(c)
					rra
					jr nc,rubberbar_sync+2


					; On first frame, VCC will overflow, and the
					; display will be fucked up. No space to waste
					; for a clean first 20ms frame =)

					; CRTC0 => Wait VCC=2 & VLC=7 (last scanline)
					; CRTC1 => Wait VCC=0 & VLC=0 (new screen)
					ld b,rubberbar_cnf_crtcsync
					djnz $

					; setup CRTC for rubberbar fx
					ld hl,&0400
					ld de,&0900	; doube usage, &0900 => CRTCReg9=0 & 256bytes sinus LUT address
					call rubberbar_crtset

					; update twister-sin pointers
					ld hl,&0131
_rubberbar_var_sinptr			equ $-2
					ld bc,&FD04
					add hl,bc
					ld (_rubberbar_var_sinptr),hl
					ld e,h
					ld h,d
					;ld h,rubberbar_data_lut_sinus256/256
					;ld d,h
					
					; rubberbar height (rasterlines)
					ld bc,287
					
					; *** 64us innerloop ***
_rubberbar_fx_loop
					; A=(sin(DE)+sin(HL)) AND 255
					ld a,(de)
					inc e
					add a,(hl)
					inc l
					exx

					; convert A into screen offset
					ld h,&0C
					ld (hl),a
					xor a
					rld
					ld d,a
					ld e,(hl)
					; AE=%xxxxPPLL.LLLL0000
					and h	;%1100 = &C
					add a,a
					add a,a
					or d
					; AE=%xxPPxxLL.LLLL0000
					ld l,a
					ld d,&0D
					call rubberbar_crtset

					exx

					; 64us rasterline loop
					dec bc
					ld a,b
					or c
					jr nz,_rubberbar_fx_loop
					
					; *** end of splitscreen ***
					;ld hl,&0907
					ld l,7
					ld de,&0402
					call rubberbar_crtset

					; loop forever
					jr rubberbar_sync
					
;*** Subroutines ***************************************************************
					
rubberbar_crtset:			; write two CRTC registers
					ld b,&BC
					out (c),h	; select register
					inc b
					out (c),l	; write register
					dec b
					out (c),d	; select register
					inc b
					out (c),e	; write register
					ret

rubberbar_generator:
					; draw 64 lines of the twister per 16k page.
					; there's 4 pages => 256 lines. Only the firsts
					; &800 bytes of each pages is used.
					ld b,64
_rubberbar_generator_16k
						;save vram pointer
						push hl
						dec l
						exx
						; save sin pointer
						push hl
						
						; read x1 = cos(a+45)
						ld a,(hl)
						ld c,a
						; set the plotter to the x1 position
						exx
						inc a
						ld c,%0000001
_rubberbar_generator_locate				rrc c
							jr nc,$+2+1
							inc l
							dec a
							jr nz,_rubberbar_generator_locate
						exx
						
						; read x2 = cos(a-45)
						dec h
						;res 2,h
						ld a,(hl)
						sub c	; x2-x1 = Line lenght
						ld b,a
_rubberbar_generator_plot
							; 8bits dither level
							ld a,(hl)
							
							;res 2,h
							
							; 3bits dither index
							rlca
							rlca
							rlca
							and %111
							; 8bits dither value
							ld e,a
							ld d,rubberbar_data_dither_seq/256
							ld a,(de)	; read dithering byte
							; plot pixel
							exx
							and c		; apply bitmask
							or (hl)		; merge with vram
							ld (hl),a
							rrc c		; next pixel, rotate bitmask
							jr nc,$+3
							inc hl		; move to next byte
							exx

							jr nc,$+3
							inc hl

							; repeat for line-lenght pixels
							djnz _rubberbar_generator_plot
_rubberbar_generator_plot_skip
						; restore sin pointer
						pop hl
						; move to next angle (0, 511)
						inc hl
						;res 2,h
						exx
						
						; move to the next line
						pop hl
						ld de,32
						add hl,de
						djnz _rubberbar_generator_16k
					; move to the next 16k page
					ld de,&3800
					add hl,de
					jr nc,rubberbar_generator
					
					; exit after 4x16k pages processed
					ret
					
					
