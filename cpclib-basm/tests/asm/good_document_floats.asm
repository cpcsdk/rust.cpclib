    ; Test floating point values and operations
    org $4000

start:
    ; Basic floating point values
    pi = 3.14159
    half = 0.5
    negative = -2.5
    
    ; Assertions for basic floats
    assert pi == 3.14159
    assert half == 0.5
    assert negative == -2.5
    
    ; Scientific notation
    micro = 1.0e-6
    thousand = 1.5e3
    
    assert micro == 0.000001
    assert thousand == 1500.0
    
    ; Float arithmetic
    sum = 1.5 + 2.5
    assert sum == 4.0
    
    product = 2.0 * 3.5
    assert product == 7.0
    
    ; Comparison operators with floats
    assert 3.5 > 2.0
    assert 2.0 >= 2.0
    assert 1.5 < 2.5
    assert 2.5 <= 2.5
    assert 2.5 == 2.5
    
    ; Float functions (if supported)
    abs_neg = abs(-3.5)
    assert abs_neg == 3.5
    
    min_val = min(1.5, 2.5)
    assert min_val == 1.5
    
    max_val = max(1.5, 2.5)
    assert max_val == 2.5
    
    ; Mixed integer and float arithmetic
    mixed = 10 + 0.5
    assert mixed == 10.5
    
    ret
