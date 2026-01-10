;;; This file tests undocumented macros

;; This macro is documented
macro documented_macro(arg1)
    ld a, {arg1}
endm

macro undocumented_macro(arg2)
    ld b, {arg2}
endm

;; This is another documented macro
macro documented_macro2(x, y)
    ld h, {x}
    ld l, {y}
endm
