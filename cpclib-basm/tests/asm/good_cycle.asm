; http://mads.atari8.info/mads_eng.html

    TICKER START count
        WAITNOPS 3
    TICKER STOP

    assert count == 3

    TICKER START count2
        nop
    TICKER STOP

    assert count2 == 1