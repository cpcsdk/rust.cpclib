

data equ [2, 1, 0]
	print data

	assert data == [2, 1, 0]
	assert list_get(data, 0) == 2
	assert list_get(data, 1) == 1
	assert list_get(data, 2) == 0

data2 = list_sublist(data, 1, 3)
	print data2

	assert data2 == [1, 0]
	assert list_get(data2, 0) == 1
	assert list_get(data2, 1) == 0

data2 = list_set(data2, 1, 3)

	; Test multiline stuff
	assert list_get( data2, 1 ) == 3
	assert list_get( \
		data2, 1 ) == 3
	assert list_get( \
		data2, \
		1 ) == 3


data2 = list_sublist( \
	data, \
	1, \
	3\
	)

function is_even n
	return (({n} % 2) == 0)
endfunction

function multiply_3_add_1 n
	return ({n} * 3 + 1)
endfunction

function sum a, b
	return {a} + {b}
endfunction

data5 equ [2, 1, 0]
data6 = list_fold( \
	list_map(list_filter(data, "is_even"), "multiply_3_add_1"), \
	10, \
	"sum") ; TODO allow use of names instead of strings

	assert data6 == 18