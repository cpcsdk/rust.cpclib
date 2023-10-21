
    org 0x1000
    defb "hell"
    defb 'o' + 0x80

    org 0x2000
    str "hello"

    org 0x3000
    db "Next one will be more complex"
    db "   \" et voila"
    db "\" et voila"

    assert memory(0x1000) == memory(0x2000)
    assert memory(0x1001) == memory(0x2001)
    assert memory(0x1002) == memory(0x2002)
    assert memory(0x1003) == memory(0x2003)