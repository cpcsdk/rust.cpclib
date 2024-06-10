

label equ  3
.label equ 2

	assert label == 3
	assert .label == 2


other: equ 5
.label2 equ 4

	assert other == 5
	assert .label2 == 4


	ifdef label.label
		fail "label.label does not exist"
	endif

	ifdef other.label
		fail"label.label does not exist"
	endif

	ifndef .label
		fail ".label must exist."
	endif

	ifndef .label2
		fail ".label2 must  exist"
	endif