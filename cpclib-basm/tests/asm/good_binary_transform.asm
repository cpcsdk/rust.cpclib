	org 0x1000
data = load("AZERTY1.TXT")
repeat 10
	data = list_extend(data, data)
endr
crunched = binary_transform(data, "LZEXO")


print data
print crunched

print list_len(data)
print list_len(crunched)

	assert list_len(data) > list_len(crunched)