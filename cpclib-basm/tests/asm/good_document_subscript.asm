    ; Indexing and slicing example
    org $4000

    ; target[i] - a single element, 0-based
    numbers = [10, 20, 30, 40]
    assert numbers[0] == 10
    assert numbers[3] == 40

    ; target[a..b] - a slice, using a range
    assert numbers[1..3] == [20, 30]

    ; target[[i, j, ...]] - gather several positions into a new list
    assert numbers[[0, 2]] == [10, 30]

    ; the same [i]/[a..b] forms work on strings too
    greeting = "hello world"
    assert greeting[0] == 'h'
    assert greeting[0..5] == "hello"

    ; and directly on a range, in O(1) - no list is ever built to answer this
    assert (0..1000000)[500000] == 500000

    ; a literal can be indexed directly, no named variable required
    assert [1, 2, 3][1] == 2

    ; subscripts chain - each [] applies to the result of the previous one
    nested = [[1, 2], [3, 4]]
    assert nested[1][0] == 3

    ; a matrix needs two indices, x (column) then y (row)
    grid = matrix_new([[1, 2], [3, 4], [5, 6]])
    assert grid[0, 0] == 1
    assert grid[1, 2] == 6

    ret
