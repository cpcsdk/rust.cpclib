    ; Test string literals and functions
    org $4000

start:
    ; Basic string literals (used with db directive)
data_string1:
    db "Hello, World!"
    
data_string2:
    db "CPC forever"
    
data_string3:
    db ""  ; empty string
    
    ; String escape sequences
data_escapes:
    db "Line 1\nLine 2"     ; newline
    db "Tab\there"          ; tab
    db "Path\\file"         ; backslash
    db "Say \"hello\""      ; quote
    
    ; String length function
    len1 = string_len("Hello")
    assert len1 == 5
    
    len2 = string_len("CPC")
    assert len2 == 3
    
    len3 = string_len("")
    assert len3 == 0
    
    ; String concatenation
    greeting = string_concat("Hello", " ", "World")
    assert string_len(greeting) == 11
    
    ; More complex concatenation
    full_greeting = string_concat("Hello", ", ", "dear ", "friend", "!")
    assert string_len(full_greeting) == 19
    
    ret
