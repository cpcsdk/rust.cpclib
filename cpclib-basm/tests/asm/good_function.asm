	;;
	; The function `name` takes 3 arguments arg1, arg2, and arg3,
	; uses a local variable
	; and returns a value (the sum of the two arguments).
	; No z80 code is allowed there, but it is possible to use some directives
	FUNCTION name, arg1, arg2, arg3

		IF {arg3} > 0
                  local1 = {arg1} + {arg2}
		ELSE
                  local1 = {arg1} - {arg2}
		ENDIF

		IF {arg1} > 2
			return local1
		ENDIF

		repeat 3
			local1 = local1+1
		rend

		return local1
		
	ENDFUNCTION
	
	; Use the function name
	ld a, name(0, 1, 2)
	assert name(0, 1, 2) == 4
	assert name(3, 3, -2) == 0

	ifdef local1
		fail "Function variables must not outlive the function"
	endif


	FUNCTION name2 arg1, arg2, arg3

		return true
		
	ENDFUNCTION

	FUNCTION name3 \
		 arg1, arg2, arg3

		return true
		
	ENDFUNCTION

		FUNCTION name3 \
		 		arg1, \
				arg2, \
				arg3

		return true
		
	ENDFUNCTION
