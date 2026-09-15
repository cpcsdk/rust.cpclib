    ; Lambda expressions example
    org $4000

    ; (params) => expr - an inline, unnamed function, most useful as a
    ; callback for list_map/list_filter/list_fold/list_position_predicate
    numbers = [1, 2, 3, 4, 5]

    doubled = list_map(numbers, (x) => x * 2)
    assert doubled == [2, 4, 6, 8, 10]

    evens = list_filter(numbers, (x) => x % 2 == 0)
    assert evens == [2, 4]

    total = list_fold(numbers, 0, (acc, x) => acc + x)
    assert total == 15

    ret
