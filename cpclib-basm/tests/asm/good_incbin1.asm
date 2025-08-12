
here
	incbin "AZERTY.TXT"
there

	assert peek(here) == 'A'
	assert peek(here+1) == 'Z'

	assert there-here == 26