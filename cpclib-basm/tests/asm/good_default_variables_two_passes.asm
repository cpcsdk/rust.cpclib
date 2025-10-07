
	ld hl, force_new_pass
	
	print BASM
	print BASM_VERSION

	ifndef BASM
		fail "BASM is defined ! and should be detected as such"
	endif

	ifdef BASM
		assert true
	else	
		assert false
	endif

	ifndef BASM_VERSION
		fail "BASM is defined ! and should be detected as such"
	endif


	ifdef BASM_VERSION
		assert true
	else	
		assert false
	endif


force_new_pass