    ; SMC offset labels example
    org $4000

    ; Manual: answer resolves to the address of the SECOND byte of this
    ; instruction (the immediate operand of `ld a,n`), not the address of
    ; the instruction itself.
start:
answer+1:
    ld a, 13
    assert answer == start + 1
    assert peek(answer) == 13

    ; Smart: same result, but the offset (1) is inferred automatically from
    ; the instruction that follows.
next_start:
next_answer+*:
    ld a, 42
    assert next_answer == next_start + 1

    ; Runtime patch, the whole point of an SMC offset label: poke a new
    ; immediate value into the instruction itself before it runs.
    ld hl, next_answer
    ld (hl), 99

    ret
