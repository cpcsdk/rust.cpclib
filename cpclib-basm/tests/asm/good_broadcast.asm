; Scalar/list broadcasting: an operator applied element-wise across a list.
; List/list (same length) already worked before this; list/scalar and
; scalar/list are the new part.

; arithmetic, both operand orders
ASSERT list_get([1,2,3] + 10, 0) = 11
ASSERT list_get([1,2,3] + 10, 2) = 13
ASSERT list_get(10 + [1,2,3], 0) = 11
ASSERT list_get([10,20,30] - 5, 0) = 5
ASSERT list_get([1,2,3] * 2, 2) = 6
ASSERT list_get([10,20,30] / 10, 1) = 2
ASSERT list_get([10,11,12] % 10, 2) = 2

; bitwise
ASSERT list_get([6,5] & 3, 0) = 2
ASSERT list_get([6,5] & 3, 1) = 1
ASSERT list_get([4,1] | 2, 0) = 6
ASSERT list_get([4,1] | 2, 1) = 3

; comparisons - broadcast into a list of bool
ASSERT list_get([1,2,3] < 2, 0) = true
ASSERT list_get([1,2,3] < 2, 1) = false
ASSERT list_get([1,2,3] < 2, 2) = false
ASSERT list_get([1,2,3] > 2, 2) = true

; a range broadcasts too, via the same materialize-at-the-boundary path
ASSERT list_get((0..3) * 2, 0) = 0
ASSERT list_get((0..3) * 2, 1) = 2
ASSERT list_get((0..3) * 2, 2) = 4

; nested lists broadcast recursively
ASSERT list_get(list_get([[1,2],[3,4]] + 1, 0), 0) = 2
ASSERT list_get(list_get([[1,2],[3,4]] + 1, 1), 1) = 5

; DB emission end to end
start:
    db (0..3) + 10
end_marker:
ASSERT peek(start) = 10
ASSERT peek(start + 1) = 11
ASSERT peek(start + 2) = 12
ASSERT end_marker - start = 3
