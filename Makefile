CRATES= cpclib-sna \
		cpclib-tokens\
		cpclib-disc\
		cpclib-basic\
		cpclib-xfer\
		cpclib-xfertool\
		cpclib-asm\
		cpclib-z80emu\
		cpclib-macros \
		cpclib
publish:
	for project in $(CRATES) ; \
	do cd $$project ; \
	   cargo +nightly publish || exit -1; \
	   cd ..; \
	done