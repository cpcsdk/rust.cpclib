
    TICKER START duration_varying_code
        xor a
        ld b, 1
    TICKER STOP
    assert duration_varying_code == 1 + 2
    UNDEF duration_varying_code


    TICKER START duration_varying_code
        xor a
    TICKER STOP
    assert duration_varying_code == 1 
    UNDEF duration_varying_code

    TICKER START duration_varying_code
    TICKER STOP
    assert duration_varying_code == 0
    UNDEF duration_varying_code


    assert duration(xor a) == 1
    ;assert duration(xor a : xor a) == 2 ; Does not compile yet Could be a good idea


    TICKER START duration_varying_code
        WAITNOPS 64
    TICKER STOP
    assert duration_varying_code == 64
    UNDEF duration_varying_code


    TICKER START duration_stable_code
        TICKER START duration_varying_code
            out (c), c
        TICKER STOP
        WAITNOPS 64 - duration_varying_code
    TICKER STOP
    assert duration_stable_code == 64
    UNDEF duration_varying_code


    MACRO BUILD_STABLE_CODE duration, r#code
        TICKER START .my_count
            {code}
        TICKER STOP
        ASSERT {duration} >= .my_count
        WAITNOPS {duration}-.my_count

        IFDEF DEBUG_EXPECTED_DURATION
            ASSERT .my_count == DEBUG_EXPECTED_DURATION
        ENDIF
        UNDEF .my_count
    ENDM

    DEBUG_EXPECTED_DURATION = 2
    BUILD_STABLE_CODE 64, "xor a : xor a"
