
	; Skip the {start} first element of list {l}
	FUNCTION SKIP, l, start
		pos = list_len({l})
		if {start} < pos
			return list_sublist({l}, {start}, pos)
		else
			return []
		endif
	ENDFUNCTION

	; Take the {amount} first element of list {l}
	FUNCTION TAKE, l, amount
		len = list_len({l})
		start = 0
		finish = start + min({amount}, len)
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

	; Various test to check appropriate behavior
	assert SKIP([1, 2, 3, 4], 2) == [3, 4]
	assert SKIP([1, 2, 3, 4], 5) == []

	assert TAKE([1, 2, 3, 4], 2) == [1, 2]
	assert TAKE([1, 2, 3, 4], 5) == [1, 2, 3, 4]

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