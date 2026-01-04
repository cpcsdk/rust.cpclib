; Test min/max with more than 2 arguments using ASSERT
; These should all pass if min/max are variadic and correct

; min tests
ASSERT min(5, 3, 7) = 3
ASSERT min(10, 2, 8, 4) = 2
ASSERT min(9, 9, 9, 9) = 9
ASSERT min(100, 50, 25, 75, 60) = 25

; max tests
ASSERT max(5, 3, 7) = 7
ASSERT max(10, 2, 8, 4) = 10
ASSERT max(9, 9, 9, 9) = 9
ASSERT max(100, 50, 25, 75, 60) = 100

; mixed order
ASSERT min(7, 5, 3, 8, 6) = 3
ASSERT max(7, 5, 3, 8, 6) = 8

; fmod
ASSERT fmod(10, 3) = 1
ASSERT fmod(25, 7) = 4

; atan2
ASSERT atan2(0, 1) = 0
ASSERT atan2(1, 0) > 1

; hypot
ASSERT hypot(3, 4) = 5

; ldexp
ASSERT ldexp(2, 3) = 16

; fdim
ASSERT fdim(7, 3) = 4
ASSERT fdim(3, 7) = 0

; fstep
ASSERT fstep(5, 3) = 1
ASSERT fstep(2, 3) = 0

; fmax/fmin
ASSERT fmax(5, 3) = 5
ASSERT fmin(5, 3) = 3

; clamp
ASSERT clamp(5, 1, 10) = 5
ASSERT clamp(0, 1, 10) = 1
ASSERT clamp(15, 1, 10) = 10

; lerp
; Commented as lerp with floating point is not yet supported
;ASSERT lerp(0, 10, 0.5) = 5
;ASSERT lerp(10, 20, 0.25) = 12.5

; isgreater/isless
ASSERT isgreater(5, 3) = 1
ASSERT isgreater(3, 5) = 0
ASSERT isless(3, 5) = 1
ASSERT isless(5, 3) = 0

; fremain
ASSERT fremain(10, 3) = 1
ASSERT fremain(25, 7) = 4
