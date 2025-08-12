
label = 1
label =3
.label=2

	assert label == 3


label = 5
	assert label == 5

label <<= 1
	assert label == 10