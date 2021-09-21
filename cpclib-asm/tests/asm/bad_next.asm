 org 0x100


data equ  $ ; read only will make fail everythoinf
	assert data == 0x100

data1 setn  data ; data1 could be modified
data2 next data, 2 ; data2 cannot be modified
data3 next data

	assert data1 == 0x100
	assert data2 == 0x101
	assert data3 == 0x103
	assert data == 0x104
