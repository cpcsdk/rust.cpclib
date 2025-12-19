; Macro performance benchmark
; Tests macro expansion with multiple parameters and many invocations

    ORG #4000

; Simple macro with 2 args
    MACRO LOAD_PAIR, reg, value
        LD {reg}, {value}
    ENDM

; Macro with 3 args and multiple lines
    MACRO ADD_THREE, val1, val2, val3
        LD A, {val1}
        ADD {val2}
        ADD {val3}
    ENDM

; Macro with string handling (r# prefix)
    MACRO DEFINE_TEXT, r#text
        DB "{text}"
        DB 0
    ENDM

; Nested operations macro
    MACRO SETUP_REG, reg, value, addr
        LD {reg}, {value}
        INC {reg}
        DEC {reg}
        LD ({addr}), {reg}
    ENDM

; Math expression macro
    MACRO CALC_OFFSET, base, offset
        LD HL, {base} + {offset} * 2
        LD DE, {base} - {offset}
    ENDM

START:
    ; Generate many macro calls to stress the expansion system
    REPEAT 100
        LOAD_PAIR A, 0
        LOAD_PAIR B, 1
        LOAD_PAIR C, 2
        ADD_THREE 10, 20, 30
        SETUP_REG HL, #1234, #5678
        CALC_OFFSET 100, 50
    REND

    ; More complex nested expansions
    REPEAT 50
        LOAD_PAIR DE, #C000
        ADD_THREE A, B, C
        LOAD_PAIR HL, DE
        SETUP_REG BC, 255, #8000
        CALC_OFFSET 200, 75
        ADD_THREE 5, 10, 15
    REND

    ; String macros
    REPEAT 30
        DEFINE_TEXT("Hello World")
        DEFINE_TEXT("Test String")
        DEFINE_TEXT("Benchmark")
    REND

    ; Heavy computation macros
    REPEAT 80
        CALC_OFFSET 1000, 100
        CALC_OFFSET 2000, 200
        SETUP_REG DE, 128, #4000
        ADD_THREE 1, 2, 3
        LOAD_PAIR A, B
    REND

    ; Final batch with all macro types
    REPEAT 60
        LOAD_PAIR HL, BC
        ADD_THREE 7, 8, 9
        SETUP_REG AF, 42, #7000
        CALC_OFFSET 500, 25
        DEFINE_TEXT "Final"
    REND

END:
    RET
