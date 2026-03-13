; Override value: troize=20 resets counter, quatre continues at 21
; Mirrors RASM test: preums=0, deuze=1, troize=20, quatre=21
	enum myprefix
preums
deuze
troize = 20
quatre
	mend
	assert myprefix_preums == 0
	assert myprefix_deuze == 1
	assert myprefix_troize == 20
	assert myprefix_quatre == 21
