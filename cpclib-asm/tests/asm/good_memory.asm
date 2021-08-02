    org 0x4000
    assert memory(label2) == 4

label1
    db 1, 2, 3

label2
    db 4, 5, 6

    assert memory(label1) == 1
    assert memory(label1+2) == 3
