	ifdef toto
		fail "BUG in assembler. toto is not yet defined"
	endif
	
	ifndef toto
		print "Great toto is not yet defined"
	endif

	toto

	ifndef toto
		fail "BUG in assembler. toto is yet defined now"
	endif
	
	ifdef toto
		print "Great toto is defined"
	endif
