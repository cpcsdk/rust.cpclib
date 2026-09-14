    ; Broadcasting example: an operator applied element-wise across a list
    org $4000

    ; arithmetic operators broadcast a scalar against every element, in
    ; either order
    a1 = [1, 2, 3] + 10
    assert a1 == [11, 12, 13]

    a2 = 10 + [1, 2, 3]
    assert a2 == [11, 12, 13]

    a3 = [10, 20, 30] - 5
    assert a3 == [5, 15, 25]

    a4 = [1, 2, 3] * 2
    assert a4 == [2, 4, 6]

    a5 = [10, 20, 30] / 10
    assert a5 == [1.0, 2.0, 3.0]

    ; bitwise AND/OR broadcast too
    b1 = [6, 5] & 3
    assert b1 == [2, 1]

    b2 = [4, 1] | 2
    assert b2 == [6, 3]

    ; comparisons broadcast into a list of booleans
    c1 = [1, 2, 3] < 2
    assert list_get(c1, 0) == true
    assert list_get(c1, 1) == false
    assert list_get(c1, 2) == false

    ; == and != do NOT broadcast - they compare the whole list at once,
    ; same as they always have, and return a single boolean
    assert ([1, 2, 3] == [1, 2, 3]) == true
    assert ([1, 2] == [1, 2, 3]) == false

    ; a range broadcasts too - it is converted to a list first, since a
    ; scaled or shifted range is no longer a contiguous range
    d1 = (0..3) * 2
    assert d1 == [0, 2, 4]

    ; two lists of the same length still combine element-wise, as before
    e1 = [1, 2, 3] + [10, 20, 30]
    assert e1 == [11, 22, 33]

    ; nested lists broadcast recursively
    f1 = [[1, 2], [3, 4]] + 1
    assert f1 == [[2, 3], [4, 5]]

    ret
