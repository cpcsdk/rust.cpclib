    ; Ternary operator example
    org $4000
    
start:
    ; Ternary in instruction
    ld a, (1 > 0) ? 42 : 0
    assert memory(start) == $3e     ; ld a, nn opcode
    assert memory(start+1) == 42    ; Should be 42 (true branch)
    
    ; Max using ternary
    ld b, (10 > 20) ? 10 : 20
    assert memory(start+2) == $06   ; ld b, nn opcode
    assert memory(start+3) == 20    ; Should be 20 (false branch, 10 < 20)
    
    ; Simple data bytes with ternary
data_start:
    db (1 > 0) ? 42 : 0
    assert memory(data_start) == 42
    
    db (0 > 1) ? 42 : 0
    assert memory(data_start+1) == 0
    
    ; Nested ternary
    db (1 > 0) ? ((2 > 1) ? 100 : 50) : 0
    assert memory(data_start+2) == 100
    
    ; With arithmetic
    db (5 * 2 > 8) ? (10 + 5) : (2 + 3)
    assert memory(data_start+3) == 15
    
    ; Boolean conditions
    db true ? 1 : 0
    assert memory(data_start+4) == 1
    
    db false ? 1 : 0
    assert memory(data_start+5) == 0
    
    ; Edge case: zero condition (falsy)
    db 0 ? 99 : 77
    assert memory(data_start+6) == 77
    
    ; Edge case: non-zero condition (truthy)
    db 5 ? 99 : 77
    assert memory(data_start+7) == 99
    
    ret