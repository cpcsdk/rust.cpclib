; UNION's whole purpose is byte/address layout, which has no meaning
; inside a FUNCTION body (which only computes a value via RETURN) -
; deliberately rejected, unlike SWITCH (pure control flow) which is fine
; there.
FUNCTION f
    UNION
        db 1
    ENDU
    RETURN 1
ENDFUNCTION
org 0x4000
db f()
