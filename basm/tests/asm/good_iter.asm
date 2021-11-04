; Glass inspiration http://www.grauw.nl/projects/glass/

    iterate value, 1, 2, 10
        add {value}
        jr nz, @no_inc
            inc c
@no_inc
		call do_stuff
    iend


    iterate value in [11, 12, 110]
        add {value}
        jr nz, @no_inc
            inc c
@no_inc
		call do_stuff
    iend

do_stuff
	ret