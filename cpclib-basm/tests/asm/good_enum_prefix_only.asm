; Prefix only: default start=0, step=1
	enum fruit
APPLE
BANANA
CHERRY
	mend
	assert fruit_APPLE == 0
	assert fruit_BANANA == 1
	assert fruit_CHERRY == 2
