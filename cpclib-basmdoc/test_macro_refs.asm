;;; Test file to verify cross-references inside macros

;; Target symbol that should be found
MY_SYMBOL equ 0x1234

;; Another symbol
OTHER_SYMBOL equ 0x5678

;; A macro that uses MY_SYMBOL inside its content
;; This should create a reference to MY_SYMBOL
macro TEST_MACRO
    ld hl, MY_SYMBOL
    ld bc, OTHER_SYMBOL
    ret
endm

;; A function that uses symbols
function test_function()
    return MY_SYMBOL + OTHER_SYMBOL
endf

;; Use the macro
    TEST_MACRO
