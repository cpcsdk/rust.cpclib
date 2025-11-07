org 0x500


    run $
    ld	hl, #C9FB
    ld	(#38), hl
    ld sp, $

    		ld bc, 0xbc00+1 : out (c), c
		ld bc, 0xbd00+0 : out (c), c

	ld hl, music_data_start
	ld de, player_cache
	call ymp_player_init

MainLoop:
    ld	b, #F5
    in	a, (c)
    rra
    jr	nc, MainLoop



    halt		; Wait to make sure the VBL is over.
    halt

		ld bc,#7f10		; Border 
		ld a,#4c
		out (c),c		; select border
		out (c),a		; in red

    call ymp_player_update

    		ld bc,#7f54
		out (c),c

    jp	MainLoop


music_data_start
    incbin MUSIC


    include "ymp.z80"
player_cache
    ; TODO specify the cache size according to the music
