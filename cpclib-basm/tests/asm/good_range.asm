; Range expressions: a..b (exclusive), a..=b (inclusive) - Rust's own range
; syntax exactly, including being empty when a > b (no auto-descending).

; length/element checks via the O(1) list_len/list_get builtins
ASSERT list_len(0..5) = 5
ASSERT list_len(0..=5) = 6
ASSERT list_get(0..5, 0) = 0
ASSERT list_get(0..5, 4) = 4
ASSERT list_len(5..1) = 0
ASSERT list_len(5..=1) = 0

; range_step_by - no dedicated stepped-range syntax, this is how a strided
; sequence is built instead
ASSERT list_len(range_step_by(0..10, 2)) = 5
ASSERT list_get(range_step_by(0..10, 2), 1) = 2

; list_sublist(a_list, a_range) - the range used as an index selector into
; an ordinary list, distinct from the existing 3-argument form
ASSERT list_get(list_sublist([10,20,30,40,50], 1..3), 0) = 20
ASSERT list_get(list_sublist([10,20,30,40,50], 1..3), 1) = 30
ASSERT list_get(list_sublist([10,20,30,40,50], 1, 3), 0) = 20

; DB/DEFW emission - a range already evaluates to a list-shaped value, so
; the existing db/dw flattening picks it up with no directive changes
start:
    db 0..4
    db 0..=4
end_marker:

ASSERT end_marker - start = 4 + 5

; a range bound can reference a forward label
db 0..count
count equ 3

; ITERATE ... IN a range - no directive changes needed either
iterate_start:
    ITERATE i IN 0..4
        db {i}
    ENDI
iterate_end:

ASSERT iterate_end - iterate_start = 4
