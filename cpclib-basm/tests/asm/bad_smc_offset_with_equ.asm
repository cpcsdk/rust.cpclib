; An SMC offset suffix (+N/+*) only makes sense on a plain label definition
; immediately followed by an instruction - combining it with a label
; modifier like EQU is a parse error, not silently ignored.
org 0x4000
a+1 EQU 5
