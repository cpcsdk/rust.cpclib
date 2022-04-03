
outer1
	jp outer2
	jp outer2.inner1


outer2
	jp .inner1
.inner1

	ifndef outer1
		fail "outer1 is wrongly undefined"
	endif

	ifndef .inner1
		fail ".inner1 is wrongly undefined"
	endif

	ifndef outer2.inner1
		fail "outer2.inner1 is wrongly undefined"
	endif

