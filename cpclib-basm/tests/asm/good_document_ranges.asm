    ; Ranges example
    org $4000

    ; a..b - exclusive of b, same semantics as Rust's own range
    r1 = 0..5
    assert list_len(r1) == 5
    assert list_get(r1, 0) == 0
    assert list_get(r1, 4) == 4

    ; a..=b - inclusive of b
    r2 = 0..=5
    assert list_len(r2) == 6
    assert list_get(r2, 5) == 5

    ; a > b is empty, not auto-descending
    r3 = 5..1
    assert list_len(r3) == 0

    ; a range bound can be any expression, including a label
    count equ 3
    r4 = 0..count
    assert list_len(r4) == 3

    ; DB/DEFW/STR emit every value in the range directly, exactly as if it
    ; had been written out by hand
start:
    db 0..4
    db 0..=4
end_marker:
    assert end_marker - start == 4 + 5

    ; ITERATE ... IN accepts a range the same way it accepts a list
iterate_start:
    iterate i in 0..4
        db {i}
    endi
iterate_end:
    assert iterate_end - iterate_start == 4

    ; range_step_by - there is no dedicated a..step..b syntax; step through
    ; a range by calling this instead
    stepped = range_step_by(0..10, 2)
    assert list_len(stepped) == 5
    assert list_get(stepped, 0) == 0
    assert list_get(stepped, 1) == 2
    assert list_get(stepped, 4) == 8

    ; list_sublist(a_list, a_range) - a range used as an index selector into
    ; an ordinary list, gathering the elements at those positions
    source = [10, 20, 30, 40, 50]
    picked = list_sublist(source, 1..3)
    assert picked == [20, 30]

    ret
