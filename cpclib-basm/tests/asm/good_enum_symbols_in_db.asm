; Use enum-generated symbols in db
	enum
KEY_UP
KEY_DOWN
KEY_LEFT
	mend
	db KEY_UP, KEY_DOWN, KEY_LEFT


    assert KEY_UP == 0
    assert KEY_DOWN == 1
    assert KEY_LEFT == 2