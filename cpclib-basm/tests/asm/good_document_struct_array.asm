; Array of struct instances: REPEAT with an explicit 0-based counter
; (REPEAT's counter defaults to 1-based - see the REPEAT directive) generates
; one uniquely-labeled instance per element; instance.field then addresses
; each element's fields directly.
STRUCT Point
    x db 0
    y db 0
ENDSTRUCT

org 0x4000
REPEAT 3, i, 0
    pt{i}: Point({i}, {i} * 10)
ENDREPEAT

ASSERT pt0.x == 0x4000
ASSERT pt1.x == 0x4002
ASSERT pt2.x == 0x4004
ASSERT peek(pt1.y) == 10
ASSERT peek(pt2.y) == 20
