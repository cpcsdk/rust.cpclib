;;
; Confined directive is inspired by confine from rasm.
; I guess confined is more  ergonomic has it does not requires to manually specify the size of the confined area

	org 0x0000

	CONFINED
		assert $ == 0
		defs 128, 0xff
	ENDCONFINED

	CONFINED
		assert $ == 256
		defs 200, 0xff
	ENDCONFINED

	CONFINED
		assert $ == 256 + 200
		defs 20, 0xff
	ENDCONFINED