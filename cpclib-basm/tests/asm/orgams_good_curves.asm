
	BYTE SIN(1)
	BYTE ABS(SIN(1))
	ld a, ABS(SIN(1))
	ld a, ABS(SIN(1)/256)


onde1:  256 ** [
	ld a, ABS(SIN(1)/256)
	]   ; Rebond  (# est le compteur interne de repétition. Ici de 0 à 255)

onde2a:  256 ** [BYTE 1]   ; Rebond  (# est le compteur interne de repétition. Ici de 0 à 255)


onde2:  256 ** [BYTE ABS(SIN(#)/256)]   ; Rebond  (# est le compteur interne de repétition. Ici de 0 à 255)
onde3:  256 ** [BYTE ABS(SIN(4*#)/[256+#+#])] ; Rebond amorti (amplitude 3 fois moins grande en fin de courbe)