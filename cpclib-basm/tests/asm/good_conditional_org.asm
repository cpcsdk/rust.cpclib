compile_test equ 1
orgTest equ 0x2d00

    if compile_test
test
    ORG orgTest
    jp $
    endif

    jp $
