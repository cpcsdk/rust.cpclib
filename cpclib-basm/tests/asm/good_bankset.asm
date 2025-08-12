	BANKSET 0

	org 0x0000
	db 1,2,3,4

	org 0x4000
	db 5,6,7,8

	org 0x8000
	db 9,10,11,12

	org 0xc000
	db 13, 14, 15, 16


	BANKSET 1
	org 0x0000
	db 10,20,30,40

	org 0x4000
	db 50,60,70,80

	org 0x8000
	db 90,100,110,120

	org 0xc000
	db 130, 140, 150, 160


	BANKSET 0
	assert memory(0x0000) == 1
	assert memory(0x4000) == 5
	assert memory(0x8000) == 9
	assert memory(0xc000) == 13

	save "good_bankset_0_0.o", 0x0000, 4
	save "good_bankset_0_1.o", 0x4000, 4
	save "good_bankset_0_2.o", 0x8000, 4
	save "good_bankset_0_3.o", 0xc000, 4

	BANKSET 1
	assert memory(0x0000) == 10
	assert memory(0x4000) == 50
	assert memory(0x8000) == 90
	assert memory(0xc000) == 130

	save "good_bankset_1_0.o", 0x0000, 4
	save "good_bankset_1_1.o", 0x4000, 4
	save "good_bankset_1_2.o", 0x8000, 4
	save "good_bankset_1_3.o", 0xc000, 4