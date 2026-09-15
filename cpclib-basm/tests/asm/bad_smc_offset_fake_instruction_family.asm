; `label+*:` rejects the whole ADD/ADC/SUB/SBC mnemonic family, even this
; perfectly ordinary, single-instruction 8-bit-immediate form - because
; some of their other forms (ADD DE,BC-style 16-bit arithmetic) can expand
; into more than one real instruction, and `+*` only knows how to place a
; patch point inside a single already-assembled instruction. See
; docs/basm/syntax.md's "SMC offset labels" section.
org 0x4000
a+*: add a, 5
