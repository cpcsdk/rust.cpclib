
list1 equ [0, 1, 2, 3]
list1_reverse equ list_reverse(list1)

assert list1_reverse == [3, 2, 1, 0], "list_reverse failed"