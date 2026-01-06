    ; Test boolean values and operations
    org $4000

start:
    ; Basic boolean literals
    true_val = true
    false_val = false
    
    ; Boolean assertions
    assert true_val == true
    assert false_val == false
    assert true == true
    assert false == false
    assert true != false
    
    ; Boolean in conditional expressions (ternary)
    result1 = true ? 1 : 0
    assert result1 == 1
    
    result2 = false ? 1 : 0
    assert result2 == 0
    
    ; Comparison operations return booleans
    is_greater = (10 > 5)
    assert is_greater == true
    
    is_less = (10 < 5)
    assert is_less == false
    
    is_equal = (42 == 42)
    assert is_equal == true
    
    is_not_equal = (42 != 43)
    assert is_not_equal == true
    
    ; Boolean logic with comparisons
    assert (5 > 3) == true
    assert (5 < 3) == false
    assert (5 >= 5) == true
    assert (5 <= 5) == true
    assert (5 == 5) == true
    assert (5 != 5) == false
    
    ; Combined logical expressions
    and_result = (true && true)
    assert and_result == true
    
    and_false = (true && false)
    assert and_false == false
    
    or_result = (true || false)
    assert or_result == true
    
    or_false = (false || false)
    assert or_false == false
    
    ; Negation - using NOT operator
    assert !(true) == false
    assert !(false) == true
    assert NOT(true) == false
    assert NOT(false) == true
    
    ; Boolean in data generation
data_start:
    db true ? 255 : 0
    assert memory(data_start) == 255
    
    db false ? 255 : 0
    assert memory(data_start+1) == 0
    
    ; Complex boolean expressions
    complex1 = (10 > 5) && (20 < 30)
    assert complex1 == true
    
    complex2 = (10 > 5) && (20 > 30)
    assert complex2 == false
    
    complex3 = (10 < 5) || (20 < 30)
    assert complex3 == true
    
    complex4 = (10 < 5) || (20 > 30)
    assert complex4 == false
    
    ; Truthiness of non-boolean values
    ; Non-zero is truthy
    assert (5 ? true : false) == true
    assert (1 ? true : false) == true
    assert (-1 ? true : false) == true
    
    ; Zero is falsy
    assert (0 ? true : false) == false
    
    ; Using booleans to control assembly
    DEBUG_MODE = false
    RELEASE_MODE = true
    
    assert DEBUG_MODE == false
    assert RELEASE_MODE == true
    
    ret
