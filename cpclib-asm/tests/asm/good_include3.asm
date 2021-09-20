
	include "good_labels.asm" namespace "good"

	ifndef good.outer1
		fail "Error in namespace managment"
	endif

	ifdef outer1
		fail "Error in namespace managment"
	endif

	ifndef good.outer2.inner1
		fail "Error in namespace managment"
	endif