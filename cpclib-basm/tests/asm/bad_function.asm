	;;
	; The function `name` takes 2 arguments arg1 and arg2, 
	; uses a local variable
	; and returns a value (the sum of the two arguments)
	FUNCTION name, arg1, arg2, arg3

		IF {arg3} > 0
                  local1 = {arg1} + {arg2}
		ELSE
                  local1 = {arg1} - {arg2}
		ENDIF

		IF {arg1} > 2
			return local1
		ENDIF

		
	ENDFUNCTION
	
	; Use the function name XXX Fail because of no return
	ld a, name(0, 1, 2)
