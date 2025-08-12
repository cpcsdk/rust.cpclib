
	org 0x100

	if 0 == 1
		fail "not reachable"
	elseifdef toto
		fail "not reachable"
	elseifndef toto
		print "reached"
		db 1
	else
		fail "not reachable"
	endif