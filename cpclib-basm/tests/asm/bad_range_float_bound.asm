; A range bound must be an integer - a float bound is an assembly-time
; error, not a silent truncation.
org 0x4000
db 1.5..5
