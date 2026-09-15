    ; list_position_predicate example - also locks in a fix: this used to
    ; (incorrectly) compare item == predicate(item) instead of checking
    ; whether predicate(item) is true.
    org $4000

    numbers = [5, 6, 7, 8]

    ; found: index of the first element greater than 6
    first_over_6 = list_position_predicate(numbers, (x) => x > 6)
    assert first_over_6 == 2

    ; not found: -1 when no element matches
    none_over_100 = list_position_predicate(numbers, (x) => x > 100)
    assert none_over_100 == -1

    ; a predicate that ignores its argument and always returns a truthy,
    ; non-boolean value is still a valid (always-true) predicate, so this
    ; matches the FIRST element (index 0). The old, buggy implementation
    ; instead compared each item to the literal value 7 and would have
    ; wrongly answered 2 (the index of the value 7) here.
    always_true = list_position_predicate(numbers, (x) => 7)
    assert always_true == 0

    ret
