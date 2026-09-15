; UNION/NEXTU/ENDU - several alternative member listings share the same
; starting address, like a C union. Every member always executes; $ after
; ENDU advances by the MAX size any member reached, not their sum.

; Two members sharing one address - the assembled content at the
; overlapping address is always whichever member came last.
org 0x4000
s1:
UNION
    db 1
NEXTU
    dw 2
ENDU
ASSERT $ == s1 + 2
ASSERT peek(s1) == 2
ASSERT peek(s1 + 1) == 0

; Size is the true MAX across three members, not their sum.
s2:
UNION
    db 1
NEXTU
    ds 5
NEXTU
    dw 2
ENDU
ASSERT $ == s2 + 5

; rgbds' own canonical example, translated: labels in different members
; alias the same address. basm's own single-colon label syntax, not
; rgbds' `::` export marker (basm doesn't parse `::` as part of a label
; definition).
UNION
    wName: ds 10
NEXTU
    wHealth: ds 2
    wVideoBuffer: ds 16
ENDU
ASSERT wName == wHealth
ASSERT wVideoBuffer == wName + 2

; DS-only member - the strict rgbds shape.
s3:
UNION
    ds 4
NEXTU
    ds 8
ENDU
ASSERT $ == s3 + 8

; A struct instantiation member - the broadened shape (rgbds itself only
; allows DS; here any data directive is allowed).
STRUCT Point
    px db 0
    py db 0
ENDSTRUCT

s4:
UNION
    ds 4
NEXTU
    pt: Point(void)
ENDU
ASSERT s4 == pt
ASSERT pt.px == s4
ASSERT pt.py == s4 + 1
ASSERT $ == s4 + 4

; Single member, no NEXTU at all - legal, if pointless (nothing overlaps).
s5:
UNION
    db 1, 2
ENDU
ASSERT $ == s5 + 2

; An empty member is legal (size 0).
s6:
UNION
NEXTU
    db 1
ENDU
ASSERT $ == s6 + 1

; A UNION can nest inside a UNION member.
s7:
UNION
    ds 1
NEXTU
    UNION
        db 1
    NEXTU
        dw 2
    ENDU
ENDU
ASSERT $ == s7 + 2

ret
