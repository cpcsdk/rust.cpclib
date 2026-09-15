    ; Lambda expressions example - same as good_document_lambda.asm, but
    ; numbers is built from a range instead of a list literal, and the
    ; multiplier is a true global (FACTOR) instead of a literal, showing a
    ; lambda body sees real globals just like a FUNCTION would.
    org $4000

    FACTOR equ 2

    ; (params) => expr - an inline, unnamed function, most useful as a
    ; callback for list_map/list_filter/list_fold/list_position_predicate
    numbers = (1..=5)

    doubled = list_map(numbers, (x) => x * FACTOR)
    assert doubled == [2, 4, 6, 8, 10]

    evens = list_filter(numbers, (x) => x % 2 == 0)
    assert evens == [2, 4]

    total = list_fold(numbers, 0, (acc, x) => acc + x)
    assert total == 15

    ret
