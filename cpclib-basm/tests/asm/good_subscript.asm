; Postfix indexing/slicing: target[i] / target[a..b] / target[x, y]

; single index - list, string, range
l = [10, 20, 30, 40]
ASSERT l[0] = 10
ASSERT l[3] = 40

s = "hello"
ASSERT s[0] = 'h'
ASSERT s[4] = 'o'

r = 0..1000000
ASSERT r[500000] = 500000

; range index - slicing, list and string
ASSERT l[1..3] = [20, 30]
ASSERT s[1..4] = "ell"

; list-of-indices index - gathers several positions at once
ASSERT l[[0, 2]] = [10, 30]

; a literal can be subscripted directly, no named variable required
ASSERT [1, 2, 3][1] = 2
ASSERT "abc"[2] = 'c'
ASSERT (0..5)[2] = 2

; chaining
nested = [[1, 2], [3, 4]]
ASSERT nested[1][0] = 3
ASSERT nested[0][1] = 2

; binds tighter than arithmetic
a = [1, 2]
b = [10, 20]
ASSERT a[0] + b[1] = 21

; matrix[x, y] - x is column, y is row
m = matrix_new([[1, 2], [3, 4], [5, 6]])
ASSERT m[0, 0] = 1
ASSERT m[1, 0] = 2
ASSERT m[0, 2] = 5
ASSERT m[1, 2] = 6

; a macro parameter can be indexed too, when the call passes a list literal
; directly - `{l}` alone still spreads a list argument flat (no brackets),
; exactly as needed for `DB {l}`, but `{l}[...]` re-wraps it in brackets so
; indexing works. Both uses of the same parameter can coexist in one body.
MACRO GET_LIST_ITEM l, idx
    db {l}[{idx}]
ENDM
MACRO SPREAD_LIST l
    db {l}
ENDM
org 0x4000
GET_LIST_ITEM([10, 20, 30], 1)
ASSERT peek($ - 1) == 20
SPREAD_LIST([1, 2, 3])
ASSERT peek($ - 3) == 1
ASSERT peek($ - 2) == 2
ASSERT peek($ - 1) == 3

; a plain identifier argument that merely evaluates to a list needs no
; wrapping - it already substitutes as valid, directly indexable text.
mylist = [5, 6, 7]
GET_LIST_ITEM(mylist, 2)
ASSERT peek($ - 1) == 7
