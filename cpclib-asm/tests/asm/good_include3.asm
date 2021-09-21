
	include "good_labels.asm" namespace "good"

	ifndef good.outer1
		fail "good.outer1 is undefined"
	endif

	ifdef outer1
		fail "outer1 is defined"
	endif

	ifndef good.outer2.inner1
		fail "good.outer2.inner1 is undedined"
	endif