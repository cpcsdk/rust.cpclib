	assert list_len(assemble("")) == 0
	assert list_len(assemble(" nop")) == 1
	assert list_len(assemble(" nop : nop ")) == 2