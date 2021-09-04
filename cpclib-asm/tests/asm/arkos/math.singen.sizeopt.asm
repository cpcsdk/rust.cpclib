;    _____         __
;   /  _  \_______|  | ______  ______
;  /  /_\  \_  __ \  |/ /  _ \/  ___/
; /    |    \  | \/    <  <_> )___ \
; \____|__  /__|  |__|_ \____/____  >
;         \/           \/         \/
; Generate a 1024 bytes long, 8bits, sinus-like curve.
; Grim/Arkos^Semilanceata

; Configuration
;cnf_math_singen_sizeopt_store	equ &2000
;cnf_math_singen_unsigned	equ 0

					; **********************************
					; * GENERATE 1024Bytes SINUS CURVE *
					; **********************************
					LET def_math_singen_sizeopt_store	= cnf_math_singen_sizeopt_store AND &FC00
					
math_singen_sizeopt:
					; Where is stored the sinus curve used as reference (1024bytes)
					; Address must be (and will be forced) at any &400 boundary.

					; autoconfig / do not change anything below
					LET def_math_singen_sizeopt_lenght 	= 1024
					LET def_math_singen_sizeopt_store_h	= def_math_singen_sizeopt_store/256
					LET def_math_singen_sizeopt_and_mask	= &03
					LET def_math_singen_sizeopt_or_mask	= def_math_singen_sizeopt_store_h AND &FC
					LET def_math_singen_sizeopt_store_q3	= def_math_singen_sizeopt_store_h+2
					LET def_math_singen_sizeopt_store_q4	= def_math_singen_sizeopt_store_h+3
					; Parabolic approximation:
					; sin(a) = ( (a-1)^2 ) - 1 @a[0, pi/2]

					; used here :
					; sin(a) = a^2 @ a[0,pi/2]
		
					; [0, 2pi] => [0, 1024]
		
					; 39 bytes long / 6405 NOPs
					; try to beat diz ! =)
		
					xor a
					
					ld bc, def_math_singen_sizeopt_store_q3*256 + def_math_singen_sizeopt_store_q4
					ld l,a
					ld e,l
					exx
					
					ld b,a ; 256
					ld d,b
_math_singen_sizeopt_loop		
					ld c,b
					dec b
					ld e,b
					ld h,d
					ld l,d
_math_singen_sizeopt_square			add hl,de
					djnz _math_singen_sizeopt_square
						
					ld a,h
					exx
					
					rra
					ifndef cnf_math_singen_unsigned
						sub 128
					endif
					
					ld d,b
					ld h,c
					
					dec l
					
					ld (de),a ; 3rd Quad
					ld (hl),a ; 4th Quad
					
					cpl

					res 1,d
					res 1,h
					
					ld (de),a ; 1st Quad
					ld (hl),a ; 2nd Quad
					
					inc e
					
					exx
					
					ld b,c
					djnz _math_singen_sizeopt_loop
					
					ifndef cnf_math_singen_inline
						ret
					endif
