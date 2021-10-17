	;;
	; The function `name` takes 2 arguments arg1 and arg2, 
	; uses a local variable
	; and returns a value (the sum of the two arguments)
	FUNC name, arg1, arg2, arg3

		IF arg3 > 0
                  local1 = arg1 + arg2
		ELSE
                  local1 = arg1 - arg2
		ENDIF
		
		return local1
		
	ENDFUNC
	
	; Use the function name
	ld a, name(exp1, exp2, exp3)