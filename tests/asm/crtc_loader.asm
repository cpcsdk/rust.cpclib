;;
; CRTC demo loader
; Krusty/Benediction -- February 2019
;
; Finger crossed to be able to compile it with Benediction assembler

	LOCOMOTIVE crtc_bootstrap
HIDE_LINES 20
10 ' Can Robots Take Control
20 ' Benediction -- 2019
30 call {crtc_bootstrap}
	ENDLOCOMOTIVE

; to be set to 1 once the demo will be finished (otherwise we will wait time to show it)
ENABLE_CATART_DISPLAY equ 1

;;
; Real loader of the demo
crtc_bootstrap
	if ENABLE_CATART_DISPLAY
		call crtc_display_catart_if_needed
	endif
	ret


;;
; The cat art needs to be displayed depending on different factors:
; - This is not the right screen mode
; - This is not the right palette choice
crtc_display_catart_if_needed
	; TODO
	ret

	include 'basic.asm'