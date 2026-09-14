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
