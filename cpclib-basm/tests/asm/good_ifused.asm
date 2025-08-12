	org 0x1000
; 3 passes are needed there

	ifused toto
toto
		ret
	endif

	call toto

; test