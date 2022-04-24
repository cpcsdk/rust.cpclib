
	; Skip the {start} first element of list {l}
	FUNCTION SKIP, l, start
		len = list_len({l})
		if {start} < len
			return list_sublist({l}, {start}, len)
		else
			return []
		endif
	ENDFUNCTION

	; Take the {amount} first element of list {l}
	FUNCTION TAKE, l, amount
		assert {amount} > 0
		len = list_len({l})
		start = 0
		finish = start + min({amount}, len) ; seems to not work for un unknown reason
		if {amount} > len
			finish = len
		else
			finish = {amount}
		endif
		return list_sublist({l}, start, finish)
	ENDFUNCTION

	; Reverse list {l}
	FUNCTION REVERT, l
		new = []
		nb = list_len({l})
		for idx, 0, nb-1
			new = list_push(new, list_get({l}, nb-1-{idx}))
		endfor
		return new
	ENDFUNCTION

	assert list_len([1, 2, 3, 4]) == 4
	assert list_sublist([1, 2, 3, 4], 0, 2) == [1,2]
	assert list_sublist([1, 2, 3, 4], 0, 4) == [1, 2, 3, 4]


	; Various test to check appropriate behavior
	assert SKIP([1, 2, 3, 4], 2) == [3, 4]
	assert SKIP([1, 2, 3, 4], 5) == []

	assert TAKE([1, 2, 3, 4], 2) == [1, 2]
	assert min(4,5) == 4
	assert TAKE([1, 2, 3, 4], 4) == [1, 2, 3, 4]
	assert TAKE([1, 2, 3, 4], 10) == [1, 2, 3, 4]

	assert REVERT([1, 2, 3, 4]) == [4, 3, 2, 1]
	assert list_len(load("hello.sna")) == 4674

	assert list_len(TAKE(load("hello.sna"), 8)) == 8



	assert string_from_list(TAKE(load("hello.sna"), 8)) == "MV - SNA"

	; Write in memory 8 bytes from the given file
	snapshot = load("hello.sna")
	header_id = TAKE(snapshot, 8)
	db header_id

	; Check that memory is correctly set
	assert peek(0) == "M"
	assert peek(7) == "A"