; sjasmplus-style SMC offset labels: label+N: (manual) / label+*: (smart)

; Manual: the offset is written by hand.
foo:
    ld a, 13
answer+1:
    ld a, 13
ASSERT answer == foo + 3
ASSERT peek(answer) = 13

; Smart: the offset is inferred from the instruction that follows. Cross-
; checked against a hand-written +N label on an identical instruction.
smart_start:
smart+*:
    ld a, 13
manual_start:
manual+1:
    ld a, 13
ASSERT smart - smart_start == manual - manual_start

; Every confirmed instruction shape.
s1: a1+*: jp nz, 0x1234
ASSERT a1 == s1 + 1

s2: a2+*: ld ix, 0x1234
ASSERT a2 == s2 + 2

s3: a3+*: ld (0x1234), a
ASSERT a3 == s3 + 1

s4:
a4+*:
    jr s4
ASSERT a4 == s4 + 1

s5:
a5+*:
    djnz s5
ASSERT a5 == s5 + 1

s6: a6+*: in a, (0x12)
ASSERT a6 == s6 + 1

s7: a7+*: out (0x12), a
ASSERT a7 == s7 + 1

; Non-DDCB indexed form with an immediate - not to be confused with the
; DDCB/FDCB exception below.
s8: a8+*: ld (ix+3), 0x42
ASSERT a8 == s8 + 3

; DDCB/FDCB indexed bit-ops: the displacement sits at a fixed offset 2,
; regardless of the trailing opcode byte.
s9: a9+*: rlc (ix+2)
ASSERT a9 == s9 + 2

s10: a10+*: bit 3, (iy+1)
ASSERT a10 == s10 + 2

s11: a11+*: and 0x12
ASSERT a11 == s11 + 1

; A comment between the label and its instruction doesn't break the link.
s12:
a12+*:
; a comment
    ld a, 13
ASSERT a12 == s12 + 1
