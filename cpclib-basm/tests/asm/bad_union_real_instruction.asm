; UNION members are grammar-restricted to data directives and labels
; (ParsingState::UnionLimited) - a real Z80 instruction is rejected.
org 0x4000
UNION
    ld a, 1
ENDU
