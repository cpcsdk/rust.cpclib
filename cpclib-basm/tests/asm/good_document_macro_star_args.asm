; {*} expands every argument passed at the call, joined with ','.
; {*[indexes]} expands a subset - a single index, a list of indices, or a
; range. indexes is itself substituted before being evaluated, so it can
; reference {#} (the argument count) or a named parameter.
MACRO ALL_BYTES(...)
    db {*}
ENDM

MACRO FIRST_TWO(...)
    db {*[0..2]}
ENDM

MACRO LAST_BYTE(...)
    db {*[{#}-1]}
ENDM

org 0x4000
ALL_BYTES(1, 2, 3)
FIRST_TWO(10, 20, 30)
LAST_BYTE(100, 101, 102)

ASSERT peek(0x4000) == 1
ASSERT peek(0x4001) == 2
ASSERT peek(0x4002) == 3
ASSERT peek(0x4003) == 10
ASSERT peek(0x4004) == 20
ASSERT peek(0x4005) == 102
