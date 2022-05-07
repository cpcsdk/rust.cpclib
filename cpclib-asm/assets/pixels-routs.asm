;;
; TODO Translate comments
; TODO Add ifused directives
; TODO Add similar routines for mode 1


;;;  Retourne la valeur de l'octet permettant
;;;  de coder la valeur de l'encre en mode 0
;;;  sur le pixel de gauche
;;;
;;; Entree :
;;;	B : numero de la couleur choisie
;;;
;;; Sortie :
;;;	C : valeur de l'octet
;;;
;;; http://www.kjthacker.f2s.com/docs/graphics.html
get_byte_for_color_0_L:	
	LD C, 0
	BIT 0, B
	JR Z, get_byte_for_color_0_L_no_bit_0
	SET 7, C
	
get_byte_for_color_0_L_no_bit_0:
	BIT 1, B
	JR Z, get_byte_for_color_0_L_no_bit_1
	SET 3, C
	
	
get_byte_for_color_0_L_no_bit_1:
	BIT 2, B
	JR Z, get_byte_for_color_0_L_no_bit_2
	SET 5, C

get_byte_for_color_0_L_no_bit_2:
	BIT 3, B
	JR Z, get_byte_for_color_0_L_no_bit_3
	SET 1, C

get_byte_for_color_0_L_no_bit_3:
	;; Maintenant A contient l'octet codant la couleur
	RET


;;;  Retourne la valeur de l'octet permettant
;;;  de coder la valeur de l'encre en mode 0
;;;  sur le pixel de droite
;;;
;;; Entree :
;;;	B : numero de la couleur choisie
;;;
;;; Sortie :
;;;	C : valeur de l'octet
;;;
;;; http://www.kjthacker.f2s.com/docs/graphics.html
get_byte_for_color_0_R:	
	LD C, 0
	BIT 0, B
	JR Z, get_byte_for_color_0_R_no_bit_0
	SET 6, C
	
get_byte_for_color_0_R_no_bit_0:
	BIT 1, B
	JR Z, get_byte_for_color_0_R_no_bit_1
	SET 2, C
	
get_byte_for_color_0_R_no_bit_1:
	BIT 2, B
	JR Z, get_byte_for_color_0_R_no_bit_2
	SET 4, C

get_byte_for_color_0_R_no_bit_2:
	BIT 3, B
	JR Z, get_byte_for_color_0_R_no_bit_3
	SET 0, C

get_byte_for_color_0_R_no_bit_3:
	;; Maintenant A contient l'octet codant la couleur
	RET


;; Retourne le numero de la couleur du pixel gauche
;; depuis un octet en mode 0
;; Entree :
;;	Registre B : valeur de l'octet dans lequel recuperer la couleur
;;
;; Sortie :
;;	Registre C : Code de la couleur (0-15) 
;;; http://www.kjthacker.f2s.com/docs/graphics.html
get_color_for_byte_0_L:	
	LD C, 0
	BIT 7, B
	JP Z, get_color_for_byte_0_L_no_bit_0
	SET 0, C
	
get_color_for_byte_0_L_no_bit_0:
	BIT 3, B
	JP Z, get_color_for_byte_0_L_no_bit_1
	SET 1, C
	
get_color_for_byte_0_L_no_bit_1:
	BIT 5, B
	JP Z, get_color_for_byte_0_L_no_bit_2
	SET 2, C

get_color_for_byte_0_L_no_bit_2:
	BIT 1, B
	JP Z, get_color_for_byte_0_L_no_bit_3
	SET 3, C

get_color_for_byte_0_L_no_bit_3:
	;; Maintenant C contient le numero de la couleur
	RET

;; Retourne le numero de la couleur du pixel droit
;; depuis un octet en mode 0
;; Entree :
;;	Registre B : valeur de l'octet dans lequel recuperer la couleur
;;
;; Sortie :
;;	Registre C : Code de la couleur (0-15) 
;;; http://www.kjthacker.f2s.com/docs/graphics.html
get_color_for_byte_0_R:	
	LD C, 0
	BIT 6, B
	JP Z, get_color_for_byte_0_R_no_bit_0
	SET 0, C
	
get_color_for_byte_0_R_no_bit_0:
	BIT 2, B
	JP Z, get_color_for_byte_0_R_no_bit_1
	SET 1, C
	
get_color_for_byte_0_R_no_bit_1:
	BIT 4, B
	JP Z, get_color_for_byte_0_R_no_bit_2
	SET 2, C

get_color_for_byte_0_R_no_bit_2:
	BIT 0, B
	JP Z, get_color_for_byte_0_R_no_bit_3
	SET 3, C

get_color_for_byte_0_R_no_bit_3:
	;; Maintenant C contient le numero de la couleur
	RET
