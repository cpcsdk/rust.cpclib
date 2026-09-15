    ; Lambdas are sugar over the existing named-FUNCTION machinery, not
    ; real closures: a lambda body sees only its own parameters plus true
    ; globals, exactly like a real FUNCTION - it cannot see an enclosing
    ; function's own parameter. `x` here is `outer`'s parameter, not a
    ; global, so the lambda's reference to it must fail.
    FUNCTION outer, x
        RETURN list_map([1, 2, 3], (y) => y + x)[0]
    ENDFUNCTION

    org $4000
    db outer(5)
    ret
