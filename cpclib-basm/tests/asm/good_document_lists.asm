    ; Lists example
    org $4000
    
    ; Basic list creation
    list1 = [1, 2, 3, 4, 5]
    assert list1 == [1, 2, 3, 4, 5]
    
    ; Empty list
    list2 = []
    assert list2 == []
    
    ; list_len - get the length of a list
    len1 = list_len(list1)
    assert len1 == 5
    
    len2 = list_len(list2)
    assert len2 == 0
    
    ; list_get - get element at index (0-based)
    first = list_get(list1, 0)
    assert first == 1
    
    second = list_get(list1, 1)
    assert second == 2
    
    last = list_get(list1, 4)
    assert last == 5
    
    ; list_new - create a new list with n elements (all initialized to given value)
    list3 = list_new(3, 0)
    assert list_len(list3) == 3
    assert list_get(list3, 0) == 0
    assert list_get(list3, 1) == 0
    assert list_get(list3, 2) == 0
    
    ; list_new with non-zero initial value
    list3b = list_new(2, 42)
    assert list_get(list3b, 0) == 42
    assert list_get(list3b, 1) == 42
    
    ; list_set - set element at index
    list4 = list_set(list3, 0, 10)
    assert list_get(list4, 0) == 10
    assert list_get(list4, 1) == 0
    assert list_get(list4, 2) == 0
    
    list4 = list_set(list4, 1, 20)
    list4 = list_set(list4, 2, 30)
    assert list4 == [10, 20, 30]
    
    ; list_push - append an element to the end
    list5 = list_push(list1, 6)
    assert list_len(list5) == 6
    assert list_get(list5, 5) == 6
    
    ; list_sublist - extract a sublist (start_index, end_index - not included)
    ; list1 = [1, 2, 3, 4, 5], extract from index 1 to 4 (not included) = [2, 3, 4]
    sublist = list_sublist(list1, 1, 4)
    assert list_len(sublist) == 3
    assert list_get(sublist, 0) == 2
    assert list_get(sublist, 1) == 3
    assert list_get(sublist, 2) == 4
    
    ; list_extend - concatenate two lists
    list6 = [10, 20]
    list7 = [30, 40]
    combined = list_extend(list6, list7)
    assert list_len(combined) == 4
    assert combined == [10, 20, 30, 40]
    
    ; list_sort - sort list in ascending order
    unsorted = [5, 2, 8, 1, 9]
    sorted = list_sort(unsorted)
    assert sorted == [1, 2, 5, 8, 9]
    
    ; list_argsort - return indices that would sort the list
    indices = list_argsort(unsorted)
    assert list_len(indices) == 5
    ; indices should point to sorted order
    assert list_get(unsorted, list_get(indices, 0)) == 1
    assert list_get(unsorted, list_get(indices, 1)) == 2
    assert list_get(unsorted, list_get(indices, 4)) == 9
    
    ; Mixed types list
    mixed = [1, 2.5, 3]
    assert list_len(mixed) == 3
    assert list_get(mixed, 0) == 1
    assert list_get(mixed, 1) == 2.5
    assert list_get(mixed, 2) == 3
    
    ret