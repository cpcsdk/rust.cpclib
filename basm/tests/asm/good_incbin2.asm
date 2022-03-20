
here
	incbin "AZERTY.TXT", 2
there

	assert peek(here) == 'E'
	assert peek(here+1) == 'R'

	assert there-here == 26-2