; `label+*:` supports the real, single-instruction 8-bit-immediate forms of
; ADD/ADC/SBC/SUB (e.g. `ADD A,n`), but still rejects their fake 16-bit
; sibling (`ADD DE,rr` here) - it expands into more than one real
; instruction, and `+*` only knows how to place a patch point inside a
; single already-assembled instruction. See docs/basm/syntax.md's "SMC
; offset labels" section.
org 0x4000
a+*: add de, bc
