
list1 = [[5, 5, 5],[5, 5, 5],[5, 5, 5]]

mat1 = matrix_new(3, 3, 5)
mat2 = matrix_new([[5, 5, 5],[5, 5, 5],[5, 5, 5]])
mat3 = matrix_new(list1)

print mat1
print mat2
print mat3

assert mat1 == mat2
assert mat1 == mat3
assert mat2 == mat3