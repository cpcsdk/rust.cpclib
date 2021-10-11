
	org 0x100

	if 0 == 1
		fail "not reachable"
	else ifdef toto
		fail "not reachable"
	else ifndef toto
		print "reached"
		db 1
	else
		fail "not reachable"
	endif