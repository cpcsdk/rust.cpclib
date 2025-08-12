    iterate value in 11
        add {value}
        jr nz, @no_inc
            inc c
@no_inc
		call do_stuff
    iend