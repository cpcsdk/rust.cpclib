
	org 0x4000

	macro sum3(a, b, ...)
		db {a}, {b}, {#}
	endm

call1:
	sum3 1, 2
call2:
	sum3 1, 2, 3
call3:
	sum3 1, 2, 3, 4
call3_end:

	assert peek(call1) == 1
	assert peek(call1+1) == 2
	assert peek(call1+2) == 2

	assert peek(call2) == 1
	assert peek(call2+1) == 2
	assert peek(call2+2) == 3

	assert peek(call3) == 1
	assert peek(call3+1) == 2
	assert peek(call3+2) == 4

	assert call3_end - call3 == 3

	macro echo_extra(...)
		db {#}
		db {0}
	endm

call4:
	echo_extra 42
call5:
	echo_extra 42, 43
call5_end:

	assert peek(call4) == 1
	assert peek(call4+1) == 42

	assert peek(call5) == 2
	assert peek(call5+1) == 42

	assert call5_end - call5 == 2

	macro named_only(a, b)
		db {a}, {b}
	endm

call6:
	named_only 5, 6

	assert peek(call6) == 5
	assert peek(call6+1) == 6
