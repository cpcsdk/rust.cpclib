
; https://k1.spdns.de/Develop/Projects/zasm/Documentation/z71.htm
	org 0x100

label_100
	nop
label_101

	assert $$ == 0x101
	assert $ == 0x101

	phase 0x200

	assert $$ == 0x101
	assert $ == 0x200

label_200
	nop
label_201

	dephase
label_102

	assert label_100 == 0x100
	assert label_101 == 0x101
	assert label_102 == 0x102
	assert label_200 == 0x200
	assert label_201 == 0x201