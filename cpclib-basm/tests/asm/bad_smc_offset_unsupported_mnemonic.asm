; `label+*:` cannot infer an offset for an instruction with no patchable
; immediate/address operand - a clear error, not a silent wrong guess.
org 0x4000
a+*: nop
