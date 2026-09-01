    ; Integer division operator `//` - unlike `/` (which always promotes
    ; int/int division to a real value), `//` always truncates toward zero
    ; and always returns an integer.
    org $4000

    assert 7 // 2 == 3
    assert 8 // 4 == 2
    assert 8 // 3 == 2
    assert 1 // 3 == 0
    assert (-7) // 2 == -3      ; truncate toward zero, not floor
    assert 7 // (-2) == -3
    assert (-7) // (-2) == 3

start:
    ld a, 10 // 3                ; always an int -> no truncation warning, unlike 10/3
    assert memory(start) == $3e  ; ld a, nn opcode
    assert memory(start+1) == 3

    ret
