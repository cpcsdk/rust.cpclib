 ; http://www.aspisys.com/asm11man.htm
 
 org 0x100


data set $

data1 setn  data
data2 next data, 2
data3 next data

	assert data == 0x100
	assert data1 == 0x100
	assert data2 == 0x101
	assert data3 == 0x103