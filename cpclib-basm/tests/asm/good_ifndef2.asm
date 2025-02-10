	;;
	; Here, we want to verify that using the label before the definition does not break stuff
	; (there is a second pass that be problematic)
	
	
	ifdef toto
		fail "BUG in assembler. toto is not yet defined"
	endif
	
	ifndef toto
		print "Great toto is not yet defined"
	endif

	ld hl, toto
	toto

	ifndef toto
		fail "BUG in assembler. toto is yet defined now"
	endif
	
	ifdef toto
		print "Great toto is defined"
	endif
