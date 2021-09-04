; all expressions in crunched section must be resolved directly without the need of an additional pass


	org 0x100


	LZAPU
	db VAR ; fail because VAR is unknown
	LZCLOSE

VAR=1
