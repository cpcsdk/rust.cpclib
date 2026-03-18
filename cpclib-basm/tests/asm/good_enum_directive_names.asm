; ENUM whose field names are assembler directive keywords (cruncher names).
; They are valid because they are used as CRUNCHER_LZ4 etc. (with prefix).
	enum CRUNCHER
APLIB
EXOMIZER
LZ4
LZ48
LZ49
LZSA1
LZSA2
SHRINKLER
UPKR
ZX0
ZX0_BACKWARD
ZX7
	endenum

	assert CRUNCHER_APLIB      == 0
	assert CRUNCHER_EXOMIZER   == 1
	assert CRUNCHER_LZ4        == 2
	assert CRUNCHER_LZ48       == 3
	assert CRUNCHER_LZ49       == 4
	assert CRUNCHER_LZSA1      == 5
	assert CRUNCHER_LZSA2      == 6
	assert CRUNCHER_SHRINKLER  == 7
	assert CRUNCHER_UPKR       == 8
	assert CRUNCHER_ZX0        == 9
	assert CRUNCHER_ZX0_BACKWARD == 10
	assert CRUNCHER_ZX7        == 11
