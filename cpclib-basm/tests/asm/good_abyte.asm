
    org &4000

    abyte -31, '   '
    abyte 2, 5

    assert memory(&4000) == 1
    assert memory(&4001) == 1
    assert memory(&4002) == 1
    assert memory(&4003) == 7