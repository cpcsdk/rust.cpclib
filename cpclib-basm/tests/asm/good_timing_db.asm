
    TICKER START duration_varying_code
        db 0
    TICKER STOP
    assert duration_varying_code == 1
    UNDEF duration_varying_code

    TICKER START duration_varying_code
        db 0, 0
    TICKER STOP
    assert duration_varying_code == 2
    UNDEF duration_varying_code

    TICKER START duration_varying_code
        dw 0
    TICKER STOP
    assert duration_varying_code == 2
    UNDEF duration_varying_code

    TICKER START duration_varying_code
        dw 0, 0
    TICKER STOP
    assert duration_varying_code == 4
    UNDEF duration_varying_code
