
	function fibo nb
		if {nb} == 0
			return 0
		else if {nb} == 1
			return 1
		else
			return fibo({nb}-1) + fibo({nb}-2)
		endif

	endfunction

	assert fibo(0) == 0
	assert fibo(1) == 1
	assert fibo(2) == 1

;	assert fibo(5) == 5

;	assert fibo(10) == 55