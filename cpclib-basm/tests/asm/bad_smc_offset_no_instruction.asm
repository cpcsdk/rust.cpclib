; A `label+*:` (smart SMC offset) with nothing following in the pass has no
; instruction to infer its offset from - an assembling error, not a silent 0.
org 0x4000
a+*:
