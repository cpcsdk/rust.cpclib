; RASM test: simple ENUM with MEND terminator
; Generates symbol un=0, deux=1 (no bytes emitted by enum itself)
	nop
	enum
un
deux
	mend
