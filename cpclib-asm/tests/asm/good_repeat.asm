

COUNT = 5
I = 50

    repeat COUNT
        db COUNT
    rend

    repeat COUNT, J
        db {J}
    rend

    repeat COUNT, J, I
        db {J}
    rend


    repeat I
        add b
        jr nz, @no_inc
            inc c
@no_inc
    rend