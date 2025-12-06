;;; The aim of this file is to do stuffs.
;;; And this comment is a top file comment.
;;; This is documentation item 1

; This is not a documentation, just a comment
UNDOCUMENTED_LABEL


;; A raw label is considered to be a function.
;; Even if there is no return close to it.
;;
;; - IN: A, HL
;; - OUT: C
;; - MOD: None
;; This is documentation item 2
DOCUMENTED_LABEL1
	add (hl)
	ld c, a
	ret

;; First equ is 1
;; This is documentation item 3
MY_EQU1 equ 1


;; Second equ is 2
;; This is documentation item 4
MY_EQU2 equ 1 + MY_EQU1


;; Wait the vsync signal
;; 
;; - INPUT: {comment} A comment very useful
;; - MOD: B, A
;; - POSTCONDITION: Vsync signal is set
;; This is documentation item 5
macro WAIT_VSYNC comment
	; {comment}
	ld b, 0xf5
@vsync
	in a, (c)
	rra
	jr nc, @vsync
endm


	WAIT_VSYNC("blabla blabal")


