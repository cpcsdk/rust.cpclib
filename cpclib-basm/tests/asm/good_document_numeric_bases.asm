    ; Test various numeric base representations
    ; All of these should represent the same values
    
    org $4000

    ; === Value 255 in different bases ===
decimal_255:
    db 255              ; Decimal
    assert memory(decimal_255) == 255
    
hex_dollar_255:
    db $FF              ; Hexadecimal with $
    assert memory(hex_dollar_255) == 255
    
hex_0x_255:
    db 0xFF             ; Hexadecimal with 0x prefix
    assert memory(hex_0x_255) == 255
    
hex_hash_255:
    db #FF              ; Hexadecimal with # prefix
    assert memory(hex_hash_255) == 255
    
hex_ampersand_255:
    db &FF              ; Hexadecimal with & prefix
    assert memory(hex_ampersand_255) == 255
    
binary_255:
    db %11111111        ; Binary with %
    assert memory(binary_255) == 255
    
binary_0b_255:
    db 0b11111111       ; Binary with 0b prefix
    assert memory(binary_0b_255) == 255

octal_255:
    db 0o377            ; Octal with 0o prefix
    assert memory(octal_255) == 255

octal_at_255:
    db @377             ; Octal with @ prefix
    assert memory(octal_at_255) == 255

    ; Verify all representations are equal
    assert 255 == $FF
    assert 255 == 0xFF
    assert 255 == #FF
    assert 255 == &FF
    assert 255 == %11111111
    assert 255 == 0b11111111
    assert 255 == 0o377
    assert 255 == @377
    

    ; === Value 42 in different bases ===
decimal_42:
    db 42               ; Decimal
    assert memory(decimal_42) == 42
    
hex_dollar_42:
    db $2A              ; Hexadecimal
    assert memory(hex_dollar_42) == 42
    
hex_0x_42:
    db 0x2A             ; Hexadecimal with 0x
    assert memory(hex_0x_42) == 42
    
binary_42:
    db %00101010        ; Binary
    assert memory(binary_42) == 42
    
binary_0b_42:
    db 0b101010         ; Binary (without leading zeros)
    assert memory(binary_0b_42) == 42

octal_42:
    db 0o52             ; Octal
    assert memory(octal_42) == 42

    ; Verify all representations are equal
    assert 42 == $2A
    assert 42 == 0x2A
    assert 42 == #2A
    assert 42 == &2A
    assert 42 == %101010
    assert 42 == 0b101010
    assert 42 == 0o52
    assert 42 == @52


    ; === Value 4096 (16-bit) in different bases ===
decimal_4096:
    dw 4096             ; Decimal
    assert memory(decimal_4096) == 0x00  ; LSB
    assert memory(decimal_4096+1) == 0x10  ; MSB
    
hex_4096:
    dw $1000            ; Hexadecimal
    assert memory(hex_4096) == 0x00
    assert memory(hex_4096+1) == 0x10
    
binary_4096:
    dw %0001000000000000  ; Binary
    assert memory(binary_4096) == 0x00
    assert memory(binary_4096+1) == 0x10

octal_4096:
    dw 0o10000          ; Octal
    assert memory(octal_4096) == 0x00
    assert memory(octal_4096+1) == 0x10

    ; Verify all representations are equal
    assert 4096 == $1000
    assert 4096 == 0x1000
    assert 4096 == #1000
    assert 4096 == &1000
    assert 4096 == %0001000000000000
    assert 4096 == 0b1000000000000
    assert 4096 == 0o10000
    assert 4096 == @10000


    ; === Edge cases ===
    
    ; Zero in all bases
    db 0, $0, 0x0, #0, &0, %0, 0b0, 0o0, @0
    assert 0 == $0
    assert 0 == %0
    assert 0 == 0o0
    
    ; One in all bases
    db 1, $1, 0x1, #1, &1, %1, 0b1, 0o1, @1
    assert 1 == $1
    assert 1 == %1
    assert 1 == 0o1
    
    ; Powers of 2
    assert 128 == $80
    assert 128 == %10000000
    assert 128 == 0o200
    
    assert 256 == $100
    assert 256 == %100000000
    assert 256 == 0o400
    
    ret
