; limits and protected are deactivated inside crunch section as we consider they have to be applied to the crunched version


	org 0x100

	limit 0x1e0

	db "Before crunched section"

	LZAPU
		; Here, the limit does not hold because it is checked AFTER compression
		defs 0x100
		assert $> 0x1e0, "Must be allowed before compression"
	LZCLOSE

	db "After crunched section"
