
    TICKER START duration_varying_code
        nop
    TICKER STOP
    assert duration_varying_code == 1
    UNDEF duration_varying_code

    TICKER START duration_varying_code
        defs 0
    TICKER STOP
    assert duration_varying_code == 0
    UNDEF duration_varying_code


    TICKER START duration_varying_code
        defs 1
    TICKER STOP
    assert duration_varying_code == 1
    UNDEF duration_varying_code


    TICKER START duration_varying_code
        defs 2
    TICKER STOP
    assert duration_varying_code == 2
    UNDEF duration_varying_code

    

    

    