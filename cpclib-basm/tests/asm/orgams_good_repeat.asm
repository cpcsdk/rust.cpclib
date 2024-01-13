	org #300

	5 ** inc l
	10 ** [
		ld hl, 0
		ld de, 0
	]

slots# = 5
	slots# ** dec e
	slots# ** [
		dec e
		xor a
	]


bob_w = 5

	      (bob_w - 2) ** [
          ld e,(hl):ld a,(de):ld (hl),a:inc l
          ]

	bob_w - 2 ** [
          ld e,(hl):ld a,(de):ld (hl),a:inc l
          ]


parallax#=10
	parallax# ** [
		byte 30 +#*2
	]