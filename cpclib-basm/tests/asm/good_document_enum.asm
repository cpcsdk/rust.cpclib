    ; ENUM directive — basic usage (auto-numbered from 0)
    enum
KEY_UP
KEY_DOWN
KEY_LEFT
KEY_RIGHT
    mend

    assert KEY_UP    == 0
    assert KEY_DOWN  == 1
    assert KEY_LEFT  == 2
    assert KEY_RIGHT == 3

    ; ENUM with prefix, start value and step
    enum color, 2, 2
red
green
blue
    endenum

    assert color_red   == 2
    assert color_green == 4
    assert color_blue  == 6

    ; ENUM with value override — resets counter
    enum myenum
first
second
third = 10
fourth
    mend

    assert myenum_first  == 0
    assert myenum_second == 1
    assert myenum_third  == 10
    assert myenum_fourth == 11
